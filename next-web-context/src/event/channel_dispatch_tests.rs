//! The tests of the two dispatch kinds of a listener.
//!
//! The tests publish through the multicaster the way an application does — the
//! synchronous `multicast_event` of the trait — and hold what each kind
//! guarantees: an inline listener has run when the publishing call returns, a
//! channel dispatched one has had the event queued, and the queue delivers the
//! events in order, refuses the ones it cannot hold, isolates a listener that
//! panics, and is drained by a shutdown.

use std::{
    any::{Any, TypeId},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::{Notify, Semaphore};

use crate::{
    ApplicationEvent, ApplicationListener, BoxFuture,
    event::{
        ApplicationEventMulticaster, ApplicationListenerRegistration, ChannelDispatch,
        DefaultApplicationEventMulticaster, ErasedApplicationListener, ListenerDispatch,
        ListenerFailurePolicy, MulticastError, MulticastErrorHandler,
    },
};

/// The event of the tests, which carries the number of the event.
#[derive(Clone, Debug)]
struct NumberedEvent(u8);

impl ApplicationEvent for NumberedEvent {
    fn source(&self) -> &dyn Any {
        self
    }

    fn event_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn source_type(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// A listener that records the numbers of the events it receives.
#[derive(Default)]
struct NumberedListener {
    received: Arc<Mutex<Vec<u8>>>,
}

impl NumberedListener {
    /// Returns a listener and the list it records into.
    fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
        let received = Arc::new(Mutex::new(Vec::new()));

        (
            Self {
                received: Arc::clone(&received),
            },
            received,
        )
    }

    /// Returns the numbers the listener recorded.
    fn received(received: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
        received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl ApplicationListener<Arc<dyn ApplicationEvent>> for NumberedListener {
    fn on_application_event<'a>(&'a self, event: &'a Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(event) = (event.as_ref() as &dyn Any).downcast_ref::<NumberedEvent>() {
                self.received
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(event.0);
            }
        })
    }
}

/// A listener that waits for a permit before it records the event.
///
/// It is what makes the tests of the queue deterministic: the event is delivered
/// only when the test allows it, and it tells the test when the worker started on
/// an event, so what the queue accepted and what it refused does not depend on how
/// fast the worker is.
struct GatedListener {
    received: Arc<Mutex<Vec<u8>>>,
    gate: Arc<Semaphore>,
    started: Arc<Notify>,
}

impl ApplicationListener<Arc<dyn ApplicationEvent>> for GatedListener {
    fn on_application_event<'a>(&'a self, event: &'a Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.started.notify_one();
            let _permit = self.gate.acquire().await;
            if let Some(event) = (event.as_ref() as &dyn Any).downcast_ref::<NumberedEvent>() {
                self.received
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(event.0);
            }
        })
    }
}

/// A listener that panics for every event.
struct PanickingListener;

impl ApplicationListener<Arc<dyn ApplicationEvent>> for PanickingListener {
    fn on_application_event<'a>(&'a self, _event: &'a Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            panic!("the listener of the tests failed on purpose");
        })
    }
}

/// The error handler of the tests, which counts the failures it is given.
#[derive(Default)]
struct CountingErrorHandler {
    failures: AtomicUsize,
}

impl MulticastErrorHandler for CountingErrorHandler {
    fn handle_listener_failure(&self, _listener_id: &str, _message: &str) {
        self.failures.fetch_add(1, Ordering::SeqCst);
    }
}

/// Returns the multicaster of the tests.
fn multicaster() -> (DefaultApplicationEventMulticaster, Arc<CountingErrorHandler>) {
    let handler = Arc::new(CountingErrorHandler::default());
    let multicaster = DefaultApplicationEventMulticaster::new_boxed()
        .with_error_handler(Arc::clone(&handler) as Arc<dyn MulticastErrorHandler>);

    (multicaster, handler)
}

/// Registers a listener that is dispatched over a channel of `capacity` events.
fn register_channel(
    multicaster: &DefaultApplicationEventMulticaster,
    id: &str,
    listener: ErasedApplicationListener,
    capacity: usize,
) {
    multicaster.register(
        ApplicationListenerRegistration::new(id, listener)
            .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(
                capacity,
            ))),
    );
}

/// Publishes an event with the given number.
fn publish(multicaster: &DefaultApplicationEventMulticaster, number: u8) -> Result<(), MulticastError> {
    multicaster.multicast_event(Arc::new(NumberedEvent(number)))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_inline_listener_runs_before_the_publishing_call_returns() {
    let (multicaster, _handler) = multicaster();
    let (listener, received) = NumberedListener::new();
    multicaster.register(ApplicationListenerRegistration::new(
        "inline",
        Arc::new(listener),
    ));

    publish(&multicaster, 1).expect("the event is published");

    assert_eq!(
        NumberedListener::received(&received),
        vec![1],
        "the inline listener ran as part of the publishing call"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_channel_listener_is_delivered_after_the_publishing_call_returns() {
    let (multicaster, _handler) = multicaster();
    let received = Arc::new(Mutex::new(Vec::new()));
    let gate = Arc::new(Semaphore::new(0));
    register_channel(
        &multicaster,
        "channel",
        Arc::new(GatedListener {
            received: Arc::clone(&received),
            gate: Arc::clone(&gate),
            started: Arc::new(Notify::new()),
        }),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    publish(&multicaster, 1).expect("the event is published");

    assert!(
        NumberedListener::received(&received).is_empty(),
        "the channel listener had not run when the publishing call returned"
    );

    gate.add_permits(1);
    multicaster.flush_async().await;

    assert_eq!(
        NumberedListener::received(&received),
        vec![1],
        "the event was delivered once the queue was flushed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_kinds_receive_the_event_of_one_publishing_call() {
    let (multicaster, _handler) = multicaster();
    let (inline, inline_received) = NumberedListener::new();
    let (channel, channel_received) = NumberedListener::new();
    multicaster.register(ApplicationListenerRegistration::new(
        "inline",
        Arc::new(inline),
    ));
    register_channel(
        &multicaster,
        "channel",
        Arc::new(channel),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    publish(&multicaster, 7).expect("the event is published");

    assert_eq!(NumberedListener::received(&inline_received), vec![7]);
    assert!(NumberedListener::received(&channel_received).is_empty());

    multicaster.flush_async().await;

    assert_eq!(NumberedListener::received(&channel_received), vec![7]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_events_of_a_channel_listener_are_delivered_in_order() {
    let (multicaster, _handler) = multicaster();
    let (listener, received) = NumberedListener::new();
    register_channel(
        &multicaster,
        "channel",
        Arc::new(listener),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    for number in 0..50 {
        publish(&multicaster, number).expect("the event is published");
    }
    multicaster.flush_async().await;

    assert_eq!(
        NumberedListener::received(&received),
        (0..50).collect::<Vec<u8>>(),
        "the queue delivers the events one after the other, in the order they were published"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_queue_refuses_the_event_and_reports_it() {
    let (mut multicaster, handler) = multicaster();
    multicaster.set_error_handler(Arc::clone(&handler) as Arc<dyn MulticastErrorHandler>);
    multicaster = multicaster.with_failure_policy(ListenerFailurePolicy::Propagate);
    let received = Arc::new(Mutex::new(Vec::new()));
    let gate = Arc::new(Semaphore::new(0));
    let started = Arc::new(Notify::new());
    register_channel(
        &multicaster,
        "channel",
        Arc::new(GatedListener {
            received: Arc::clone(&received),
            gate: Arc::clone(&gate),
            started: Arc::clone(&started),
        }),
        1,
    );

    publish(&multicaster, 1).expect("the first event fits in the queue");
    // The worker takes the event out of the queue and waits on the gate, so the
    // queue is empty and the next event fits in it, while the one after that does
    // not.
    started.notified().await;
    publish(&multicaster, 2).expect("the event fits in the queue of the listener");
    let refused = publish(&multicaster, 3).expect_err("the queue of the listener is full");

    assert!(
        matches!(refused, MulticastError::ListenerFailures(_)),
        "the refused event is reported as a failure of the publishing call, it was: {refused:?}"
    );
    assert_eq!(
        handler.failures.load(Ordering::SeqCst),
        1,
        "the refusal is reported to the error handler"
    );
    let statistics = multicaster
        .channel_statistics("channel")
        .expect("the listener is dispatched over a channel");
    assert_eq!(statistics.sent, 2);
    assert_eq!(statistics.rejected, 1);

    gate.add_permits(2);
    multicaster.flush_async().await;
    assert_eq!(
        NumberedListener::received(&received),
        vec![1, 2],
        "the refused event is not delivered later"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reporting_multicaster_does_not_fail_the_publishing_call_when_the_queue_is_full() {
    let (multicaster, _handler) = multicaster();
    let received = Arc::new(Mutex::new(Vec::new()));
    let gate = Arc::new(Semaphore::new(0));
    register_channel(
        &multicaster,
        "channel",
        Arc::new(GatedListener {
            received: Arc::clone(&received),
            gate: Arc::clone(&gate),
            started: Arc::new(Notify::new()),
        }),
        1,
    );

    publish(&multicaster, 1).expect("the first event fits in the queue");
    publish(&multicaster, 2).expect("the default policy reports the refusal and publishes");

    assert_eq!(
        multicaster
            .channel_statistics("channel")
            .expect("the listener is dispatched over a channel")
            .rejected,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_listener_that_panics_does_not_stop_the_queue() {
    let (multicaster, handler) = multicaster();
    multicaster.register(
        ApplicationListenerRegistration::new("panicking", Arc::new(PanickingListener))
            .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(4))),
    );

    publish(&multicaster, 1).expect("the event is published");
    multicaster.flush_async().await;

    let statistics = multicaster
        .channel_statistics("panicking")
        .expect("the listener is dispatched over a channel");
    assert_eq!(statistics.delivered, 0);
    assert_eq!(statistics.failed, 1);
    assert_eq!(
        handler.failures.load(Ordering::SeqCst),
        1,
        "the failure of the listener is reported once"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_shutdown_delivers_the_events_that_are_queued() {
    let (multicaster, _handler) = multicaster();
    let (listener, received) = NumberedListener::new();
    register_channel(
        &multicaster,
        "channel",
        Arc::new(listener),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    for number in 0..10 {
        publish(&multicaster, number).expect("the event is published");
    }
    multicaster.close_async().await;

    assert_eq!(
        NumberedListener::received(&received),
        (0..10).collect::<Vec<u8>>(),
        "the shutdown drains the queue before the worker stops"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_removed_listener_stops_receiving_events() {
    let (multicaster, _handler) = multicaster();
    let (listener, received) = NumberedListener::new();
    register_channel(
        &multicaster,
        "channel",
        Arc::new(listener),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    publish(&multicaster, 1).expect("the event is published");
    multicaster.flush_async().await;
    multicaster.remove_application_listener(String::from("channel"));
    publish(&multicaster, 2).expect("the event is published");
    multicaster.flush_async().await;

    assert_eq!(
        NumberedListener::received(&received),
        vec![1],
        "the listener that was removed did not receive the event that followed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_statistics_report_what_was_queued_and_delivered() {
    let (multicaster, _handler) = multicaster();
    let (listener, _received) = NumberedListener::new();
    register_channel(
        &multicaster,
        "channel",
        Arc::new(listener),
        ChannelDispatch::DEFAULT_CAPACITY,
    );

    publish(&multicaster, 1).expect("the event is published");
    multicaster.flush_async().await;

    let statistics = multicaster
        .channel_statistics("channel")
        .expect("the listener is dispatched over a channel");
    assert_eq!(statistics.sent, 1);
    assert_eq!(statistics.delivered, 1);
    assert_eq!(statistics.pending(), 0);
    assert_eq!(statistics.rejected, 0);
    assert_eq!(statistics.failed, 0);
    assert_eq!(
        multicaster.channel_listener_ids(),
        vec![String::from("channel")],
        "the multicaster reports the listeners it dispatches over a channel"
    );
}
