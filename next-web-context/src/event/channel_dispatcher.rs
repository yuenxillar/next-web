//! The worker that delivers the events of a channel dispatched listener.
//!
//! A listener that is dispatched over a channel has a queue of its own and a
//! worker that reads it, one event after the other. The queue is bounded and the
//! call that publishes an event only enqueues it, so a listener that is slow
//! cannot slow down the request that published the event, and cannot make the
//! other listeners of the event wait for it either.
//!
//! The worker is started on the runtime of the first call that publishes an event
//! to it: a multicaster is created without a runtime as often as inside one, and a
//! queue that was never published to has no reason to hold a task. When the
//! context that owns the multicaster closes, [`ChannelDispatcher::shutdown`]
//! drains the queue before the worker stops, so a shutdown does not lose the
//! events an application already published.

use std::{
    fmt::{self, Debug},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{
    runtime::Handle,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    ApplicationEvent,
    event::{
        ChannelDispatch, ChannelStatistics, ErasedApplicationListener, MulticastErrorHandler,
        execution::CatchUnwind,
    },
};

/// How long a shutdown waits for the events a listener still has to deliver.
///
/// A worker drains its queue when the multicaster closes, but an application
/// cannot hang on the shutdown of a listener that is stuck: the wait is bounded,
/// and what is left is reported, so that the listener is visible instead of
/// holding the process open.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// The message the worker of a listener reads.
enum ChannelMessage {
    /// An event the listener has to run for.
    Event(Arc<dyn ApplicationEvent>),
    /// Reaches the worker after the events sent before it, so the sender knows
    /// those events were delivered.
    Barrier(oneshot::Sender<()>),
}

/// The outcome of handing an event to the queue of a listener.
pub(crate) enum SendOutcome {
    /// The queue accepted the event.
    Queued,
    /// The queue was full, so the event was dropped.
    Rejected,
    /// The worker cannot be started because there is no runtime to start it on.
    ///
    /// The caller runs the listener itself in that case, so that an event is
    /// never lost because of where it was published.
    NoRuntime,
    /// The worker is gone, so nothing will ever read the queue again.
    Closed,
}

/// The state of the worker of a listener.
#[derive(Default)]
struct Worker {
    /// The queue, until the worker starts reading it.
    receiver: Option<mpsc::Receiver<ChannelMessage>>,
    /// The task that reads the queue, once it was started.
    handle: Option<JoinHandle<()>>,
    /// Whether the worker was asked to stop.
    closed: bool,
}

/// A queue of an event listener, and the worker that reads it.
pub(crate) struct ChannelDispatcher {
    id: String,
    settings: ChannelDispatch,
    listener: ErasedApplicationListener,
    /// The writer of the queue, until the dispatcher is closed.
    sender: Mutex<Option<mpsc::Sender<ChannelMessage>>>,
    worker: Mutex<Worker>,
    statistics: Arc<ChannelStatistics>,
    error_handler: Arc<dyn MulticastErrorHandler>,
}

impl Debug for ChannelDispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChannelDispatcher")
            .field("id", &self.id)
            .field("capacity", &self.settings.capacity())
            .field("statistics", &self.statistics.snapshot())
            .finish_non_exhaustive()
    }
}

impl ChannelDispatcher {
    /// Creates the queue of the listener.
    ///
    /// The worker is not started here: it is started by the first event that is
    /// published to the queue, on the runtime that publishes it.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier the listener is registered under.
    /// * `listener` - The listener that reads the queue.
    /// * `settings` - The capacity and the policy of the queue.
    /// * `error_handler` - Where the failure of the listener is reported.
    pub(crate) fn new(
        id: String,
        listener: ErasedApplicationListener,
        settings: ChannelDispatch,
        error_handler: Arc<dyn MulticastErrorHandler>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(settings.capacity());

        Self {
            id,
            settings,
            listener,
            sender: Mutex::new(Some(sender)),
            worker: Mutex::new(Worker {
                receiver: Some(receiver),
                handle: None,
                closed: false,
            }),
            statistics: Arc::new(ChannelStatistics::default()),
            error_handler,
        }
    }

    /// Returns the identifier the listener is registered under.
    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    /// Returns the counters of the listener.
    pub(crate) fn statistics(&self) -> Arc<ChannelStatistics> {
        Arc::clone(&self.statistics)
    }

    /// Hands an event to the queue of the listener.
    ///
    /// # Arguments
    ///
    /// * `event` - The event the listener has to run for.
    pub(crate) fn send(&self, event: Arc<dyn ApplicationEvent>) -> SendOutcome {
        if let Err(outcome) = self.ensure_started() {
            return outcome;
        }

        let Some(sender) = self.sender_clone() else {
            return SendOutcome::Closed;
        };

        match sender.try_send(ChannelMessage::Event(event)) {
            Ok(()) => {
                self.statistics.record_sent();

                SendOutcome::Queued
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.statistics.record_rejected();
                self.error_handler.handle_listener_failure(
                    &self.id,
                    "the queue of the listener is full, so the event was refused",
                );

                SendOutcome::Rejected
            }
            Err(mpsc::error::TrySendError::Closed(_)) => SendOutcome::Closed,
        }
    }

    /// Waits until the events published to the queue before the call have been
    /// delivered.
    ///
    /// The barrier is queued after the events, so it can only be queued once
    /// there is room for it: the wait therefore also covers a queue that was full
    /// when the call started. Returns `false` when the listener has no worker to
    /// wait for, because no event was ever published to it.
    pub(crate) async fn flush(&self) -> bool {
        let Some(sender) = self.sender_clone() else {
            return false;
        };
        let (acknowledgement, received) = oneshot::channel();
        if sender
            .send(ChannelMessage::Barrier(acknowledgement))
            .await
            .is_err()
        {
            return false;
        }

        // The barrier is answered by the worker after the events before it, so
        // waiting for it is waiting for the delivery of those events.
        tokio::time::timeout(DRAIN_TIMEOUT, received).await.is_ok()
    }

    /// Stops the worker once the events in the queue have been delivered.
    ///
    /// The wait is bounded by [`DRAIN_TIMEOUT`]: a listener that is stuck is
    /// reported rather than keeping the shutdown of the application open.
    pub(crate) async fn shutdown(&self) {
        // Dropping the writer is what ends the worker: it reads the queue until
        // it is empty and the writer is gone, which drains the events that are
        // still in it.
        self.sender.lock().unwrap_or_else(poisoned).take();
        let handle = {
            let mut worker = self.worker.lock().unwrap_or_else(poisoned);
            worker.closed = true;
            worker.receiver.take();
            worker.handle.take()
        };
        let Some(handle) = handle else {
            return;
        };

        match tokio::time::timeout(DRAIN_TIMEOUT, handle).await {
            Ok(Ok(())) => {
                tracing::trace!(
                    listener = self.id,
                    statistics = %self.statistics.snapshot(),
                    "the worker of a channel dispatched listener drained its queue"
                );
            }
            Ok(Err(error)) => {
                tracing::warn!(
                    listener = self.id,
                    error = %error,
                    "the worker of a channel dispatched listener ended abruptly"
                );
            }
            Err(_) => {
                tracing::warn!(
                    listener = self.id,
                    statistics = %self.statistics.snapshot(),
                    timeout = ?DRAIN_TIMEOUT,
                    "the worker of a channel dispatched listener did not drain its queue in time"
                );
            }
        }
    }

    /// Starts the worker of the queue when it was not started yet.
    fn ensure_started(&self) -> Result<(), SendOutcome> {
        let mut worker = self.worker.lock().unwrap_or_else(poisoned);
        if worker.closed {
            return Err(SendOutcome::Closed);
        }
        if worker.handle.is_some() {
            return Ok(());
        }
        let Some(receiver) = worker.receiver.take() else {
            return Err(SendOutcome::Closed);
        };
        let Ok(runtime) = Handle::try_current() else {
            // No runtime was available when the first event was published to the
            // queue, so there is no worker to run it. The queue is kept, and the
            // next publish on a runtime starts the worker, so the listener is not
            // lost for the application.
            worker.receiver = Some(receiver);

            return Err(SendOutcome::NoRuntime);
        };

        let id = self.id.clone();
        let listener = Arc::clone(&self.listener);
        let statistics = Arc::clone(&self.statistics);
        let error_handler = Arc::clone(&self.error_handler);
        worker.handle = Some(runtime.spawn(async move {
            run_worker(id, listener, receiver, statistics, error_handler).await;
        }));

        Ok(())
    }

    /// Returns a writer of the queue, when the dispatcher is still open.
    fn sender_clone(&self) -> Option<mpsc::Sender<ChannelMessage>> {
        self.sender
            .lock()
            .unwrap_or_else(poisoned)
            .as_ref()
            .cloned()
    }
}

/// Reads the queue of a listener until it is empty and closed.
///
/// # Arguments
///
/// * `id` - The identifier the listener is registered under.
/// * `listener` - The listener that runs for the events of the queue.
/// * `receiver` - The queue to read.
/// * `statistics` - The counters of the listener.
/// * `error_handler` - Where the failure of the listener is reported.
async fn run_worker(
    id: String,
    listener: ErasedApplicationListener,
    mut receiver: mpsc::Receiver<ChannelMessage>,
    statistics: Arc<ChannelStatistics>,
    error_handler: Arc<dyn MulticastErrorHandler>,
) {
    while let Some(message) = receiver.recv().await {
        match message {
            ChannelMessage::Event(event) => {
                match CatchUnwind::new(listener.on_application_event(event)).await {
                    Ok(()) => statistics.record_delivered(),
                    Err(payload) => {
                        statistics.record_failed();
                        let message = crate::event::execution::panic_message(&payload);
                        error_handler.handle_listener_failure(&id, &message);
                    }
                }
            }
            ChannelMessage::Barrier(acknowledgement) => {
                let _ = acknowledgement.send(());
            }
        }
    }

    tracing::trace!(
        listener = id,
        statistics = %statistics.snapshot(),
        "the worker of a channel dispatched listener stopped"
    );
}

/// Returns the inner value of a lock that a listener panicked under.
///
/// A listener that panicked while the lock was held is reported by the error
/// handler of the multicaster, and the queue is still usable: the events in it
/// are delivered rather than dropped because of the poison.
fn poisoned<T>(poisoned: std::sync::PoisonError<T>) -> T {
    poisoned.into_inner()
}
