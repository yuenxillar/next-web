//! A trigger which fires repeatedly on a fixed period.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures::future::BoxFuture;
use next_web_core::Id;
use tokio::task::JoinHandle;

use crate::support::lock_util::{lock, read, write};
use crate::trigger::Trigger;
use crate::trigger::trigger_context::TriggerContext;
use crate::trigger::trigger_listener::TriggerListener;

/// The part of a timer which the trigger and its task share.
///
/// The task observes the armed flag between two ticks, so a timer which is
/// disarmed while its own task runs a transition is not aborted mid flight. The
/// generation tells a finishing task whether a newer task replaced it.
#[derive(Default)]
struct TimerControl {
    armed: AtomicBool,
    generation: AtomicU64,
}

/// A timer trigger.
///
/// The Java implementation schedules on a `Scheduler` and keeps a `Disposable`;
/// the Rust port spawns a tokio task. Disarming it from another task aborts the
/// task, while disarming it from the task itself lets the running transition
/// finish and stops the timer at its next tick.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct TimerTrigger<S, E> {
    listeners: RwLock<Arc<Vec<Arc<dyn TriggerListener>>>>,
    period: Duration,
    count: usize,
    control: Arc<TimerControl>,
    handle: Mutex<Option<JoinHandle<()>>>,
    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Clone for TimerTrigger<S, E> {
    fn clone(&self) -> Self {
        Self {
            listeners: RwLock::new(read(&self.listeners).clone()),
            period: self.period,
            count: self.count,
            control: self.control.clone(),
            handle: Mutex::new(None),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> TimerTrigger<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new timer trigger which fires forever.
    ///
    /// # Arguments
    /// * `period` - the period between firings
    pub fn new(period: Duration) -> Self {
        Self::with_count(period, 0)
    }

    /// Creates a new timer trigger with a firing limit.
    ///
    /// # Arguments
    /// * `period` - the period between firings
    /// * `count` - the number of firings, `0` means forever
    pub fn with_count(period: Duration, count: usize) -> Self {
        Self {
            listeners: RwLock::new(Arc::new(Vec::new())),
            period,
            count,
            control: Arc::new(TimerControl::default()),
            handle: Mutex::new(None),
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new timer trigger from a period in milliseconds.
    ///
    /// # Arguments
    /// * `period_millis` - the period in milliseconds
    pub fn with_period_millis(period_millis: u64) -> Self {
        Self::new(Duration::from_millis(period_millis))
    }

    /// Schedules the timer task.
    fn schedule(&self) {
        let period = self.period;
        let count = self.count;
        let listeners = read(&self.listeners).clone();
        let control = self.control.clone();
        let generation = self.control.generation.fetch_add(1, Ordering::SeqCst) + 1;

        let spawned = tokio::runtime::Handle::try_current().ok().map(|handle| {
            handle.spawn(async move {
                // `interval` completes its first tick immediately, so the first
                // tick is consumed: a timer waits for its period before it fires
                // for the first time.
                let mut ticker = tokio::time::interval(period);
                ticker.tick().await;

                let mut fired = 0;

                loop {
                    ticker.tick().await;

                    // A timer which was disarmed while its own task ran a
                    // transition stops here, with the transition complete.
                    if !control.armed.load(Ordering::SeqCst) {
                        return;
                    }

                    for listener in listeners.iter() {
                        listener.triggered().await;
                    }

                    fired += 1;
                    if count > 0 && fired >= count {
                        break;
                    }
                }

                // The fired-out timer is ready to be armed again, unless a newer
                // task replaced this one.
                if control.generation.load(Ordering::SeqCst) == generation {
                    control.armed.store(false, Ordering::SeqCst);
                }
            })
        });

        if let Some(handle) = spawned {
            *lock(&self.handle) = Some(handle);
        }
    }

    /// Cancels the timer task.
    ///
    /// A task which cancels the timer while it runs the transition is left to
    /// finish, so the transition is not dropped in the middle.
    fn cancel(&self) {
        if let Some(handle) = lock(&self.handle).take() {
            let own_task = tokio::task::try_id().is_some_and(|id| id == handle.id());

            if !own_task {
                handle.abort();
            }
        }
    }
}

impl<S, E> Id for TimerTrigger<S, E> {
    fn id(&self) -> &str {
        "timerTrigger"
    }
}

impl<S, E> Trigger<S, E> for TimerTrigger<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn evaluate<'a>(&'a self, _context: &'a dyn TriggerContext<S, E>) -> BoxFuture<'a, bool> {
        // The timer signals through the trigger listener, evaluating it does
        // not match an event.
        Box::pin(async move { false })
    }

    fn add_trigger_listener(&self, listener: Arc<dyn TriggerListener>) {
        let mut guard = write(&self.listeners);
        let mut next = Vec::with_capacity(guard.len() + 1);
        next.extend(guard.iter().cloned());
        next.push(listener);
        *guard = Arc::new(next);
    }

    fn event(&self) -> Option<&E> {
        None
    }

    fn arm(&self) {
        if self.control.armed.swap(true, Ordering::SeqCst) {
            return;
        }
        self.schedule();
    }

    fn disarm(&self) {
        if !self.control.armed.swap(false, Ordering::SeqCst) {
            return;
        }
        self.cancel();
    }
}
