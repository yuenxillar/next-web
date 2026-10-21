//! A trigger which fires repeatedly on a fixed period.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use futures::future::BoxFuture;
use next_web_core::Id;
use tokio::task::JoinHandle;

use crate::support::lock_util::{lock, read, write};
use crate::trigger::Trigger;
use crate::trigger::trigger_context::TriggerContext;
use crate::trigger::trigger_listener::TriggerListener;

/// A timer trigger.
///
/// The Java implementation schedules on a `Scheduler` and keeps a `Disposable`;
/// the Rust port spawns a tokio task and aborts it on disarm.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct TimerTrigger<S, E> {
    listeners: RwLock<Arc<Vec<Arc<dyn TriggerListener>>>>,
    period: Duration,
    count: usize,
    armed: AtomicBool,
    handle: Mutex<Option<JoinHandle<()>>>,
    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> Clone for TimerTrigger<S, E> {
    fn clone(&self) -> Self {
        Self {
            listeners: RwLock::new(read(&self.listeners).clone()),
            period: self.period,
            count: self.count,
            armed: AtomicBool::new(self.armed.load(Ordering::SeqCst)),
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
            armed: AtomicBool::new(false),
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

    /// Gets the period.
    pub fn get_period(&self) -> Duration {
        self.period
    }

    /// Gets the firing limit, `0` when the trigger fires forever.
    pub fn get_count(&self) -> usize {
        self.count
    }

    /// Schedules the timer task.
    fn schedule(&self) {
        if lock(&self.handle).is_some() {
            return;
        }

        let period = self.period;
        let count = self.count;
        let listeners = read(&self.listeners).clone();

        let spawned = tokio::runtime::Handle::try_current().ok().map(|handle| {
            handle.spawn(async move {
                let mut ticker = tokio::time::interval(period);
                let mut fired = 0;

                loop {
                    ticker.tick().await;

                    for listener in listeners.iter() {
                        listener.triggered().await;
                    }

                    fired += 1;
                    if count > 0 && fired >= count {
                        break;
                    }
                }
            })
        });

        if let Some(handle) = spawned {
            *lock(&self.handle) = Some(handle);
        }
    }

    /// Cancels the timer task.
    fn cancel(&self) {
        if let Some(handle) = lock(&self.handle).take() {
            handle.abort();
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
        if self.armed.swap(true, Ordering::SeqCst) {
            return;
        }
        self.schedule();
    }

    fn disarm(&self) {
        if !self.armed.swap(false, Ordering::SeqCst) {
            return;
        }
        self.cancel();
    }
}
