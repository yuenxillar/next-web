//! The application event example: one event, three listeners, two ways to
//! deliver it.
//!
//! An application publishes an event through the publisher of its context, and
//! the multicaster hands it to the listeners that support it. How a listener
//! receives the event is decided when it is registered, and this example has one
//! of each kind:
//!
//! - `AuditListener` is dispatched **inline**: it runs as part of the call that
//!   publishes the event, so the state it writes is there when the call returns.
//!   This is what a listener the request depends on needs; the route of the
//!   example reads the audit right after the call to show it.
//! - `NotificationListener` is dispatched over a **channel**: the publishing call
//!   only queues the event, and the listener runs on a worker of its own, one
//!   event after the other. It is the listener that would send a mail or call a
//!   webhook, and this one waits 50 milliseconds to look like one.
//! - `AlertListener` is also dispatched over a channel, with a queue of four
//!   events, and it fails for the orders that are too big. The failure is isolated:
//!   it is reported to the error handler of the multicaster and the queue keeps
//!   delivering the events that follow.
//!
//! The routes of the example:
//!
//! - `POST /events/orders/{id}/{amount}` publishes an order and reports what the
//!   inline listener wrote before the call returned, and what the channel
//!   listener has not delivered yet;
//! - `POST /events/burst/{count}/{amount}` publishes a burst, which fills the
//!   queue of the listeners that are slower than the loop, and reports the
//!   statistics of each queue: what it accepted, what it delivered, and what it
//!   refused;
//! - `GET /events` reports the audit, the notifications, and the statistics of
//!   the queues.
//!
//! Try it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin application_event
//! curl -X POST http://127.0.0.1:11000/app/events/orders/order-1/10
//! curl -X POST "http://127.0.0.1:11000/app/events/burst/50/10"
//! curl http://127.0.0.1:11000/app/events
//! ```

use std::{
    any::{Any, TypeId},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::extract::Path;
use next_web::{
    Application, NextWebApplication,
    core::BoxFuture,
    macros::bind::{get_mapping, post_mapping, singleton},
};
use next_web_context::{
    ApplicationEvent, ApplicationListener,
    event::{
        ApplicationEventMulticaster, ApplicationListenerRegistration, ChannelDispatch,
        DefaultApplicationEventMulticaster, ListenerDispatch,
    },
};

#[derive(Default)]
pub struct EventsApplication;

impl Application for EventsApplication {}

/// The event the example publishes.
#[derive(Clone, Debug)]
pub struct OrderPlaced {
    /// The identifier of the order.
    pub id: String,
    /// The amount of the order, in cents.
    pub amount: u64,
}

impl ApplicationEvent for OrderPlaced {
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

/// What the listeners of the example wrote, and the queue they read.
#[derive(Default)]
pub struct EventsState {
    audit: Mutex<Vec<String>>,
    notifications: Mutex<Vec<String>>,
    alerts: AtomicUsize,
    multicaster: OnceLock<Arc<DefaultApplicationEventMulticaster>>,
}

impl EventsState {
    /// Returns what the inline listener wrote.
    fn audit(&self) -> Vec<String> {
        self.audit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Returns what the channel listener that notifies delivered.
    fn notifications(&self) -> Vec<String> {
        self.notifications
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Returns the number of alerts the listener that fails was asked for.
    fn alerts(&self) -> usize {
        self.alerts.load(Ordering::SeqCst)
    }

    /// Returns the multicaster the listeners of the example are registered on.
    fn multicaster(&self) -> Option<Arc<DefaultApplicationEventMulticaster>> {
        self.multicaster.get().cloned()
    }

    /// Publishes an order through the multicaster of the example.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier of the order.
    /// * `amount` - The amount of the order, in cents.
    fn publish(&self, id: &str, amount: u64) {
        let Some(multicaster) = self.multicaster() else {
            return;
        };
        let event = Box::new(OrderPlaced {
            id: id.to_owned(),
            amount,
        });

        if let Err(error) = multicaster.multicast_event(event) {
            tracing::warn!(error = %error, "the order was published with failures");
        }
    }

    /// Returns the statistics of a queue of the example, as one line.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier the listener is registered under.
    fn statistics(&self, id: &str) -> Option<String> {
        self.multicaster()?
            .channel_statistics(id)
            .map(|statistics| statistics.to_string())
    }
}

/// Writes the order into the audit, as part of the publishing call.
struct AuditListener {
    state: Arc<EventsState>,
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for AuditListener {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(order) = (event.as_ref() as &dyn Any).downcast_ref::<OrderPlaced>() {
                self.state
                    .audit
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(format!("order {} at {} cents", order.id, order.amount));
            }
        })
    }
}

/// Notifies the order, slowly, on the worker of its queue.
struct NotificationListener {
    state: Arc<EventsState>,
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for NotificationListener {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Some(order) = (event.as_ref() as &dyn Any).downcast_ref::<OrderPlaced>() else {
                return;
            };
            let id = order.id.clone();

            // A mail, a webhook or a call to another service would be here: the
            // point of the queue is that the request does not wait for it.
            tokio::time::sleep(Duration::from_millis(50)).await;

            self.state
                .notifications
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(format!("notified {id}"));
        })
    }
}

/// Fails for the orders that are too big, which the multicaster isolates.
struct AlertListener {
    state: Arc<EventsState>,
}

impl ApplicationListener<Box<dyn ApplicationEvent>> for AlertListener {
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Some(order) = (event.as_ref() as &dyn Any).downcast_ref::<OrderPlaced>() else {
                return;
            };

            self.state.alerts.fetch_add(1, Ordering::SeqCst);
            if order.amount >= 1000 {
                panic!("the alert of the order {} could not be sent", order.id);
            }
        })
    }
}

/// The capacity of the queue of the listener that notifies.
const NOTIFICATION_CAPACITY: usize = 16;
/// The capacity of the queue of the listener that alerts.
const ALERT_CAPACITY: usize = 4;

/// Registers the listeners of the example on the multicaster.
///
/// The inline listener is registered on its own, and the two others with a queue
/// of their own. This is the whole difference between the two kinds: the
/// registration says how the event reaches the listener.
///
/// # Arguments
///
/// * `multicaster` - The multicaster the listeners are registered on.
/// * `state` - The state the listeners read and write.
fn register_listeners(
    multicaster: &DefaultApplicationEventMulticaster,
    state: &Arc<EventsState>,
) {
    // Inline: runs as part of the publishing call.
    multicaster.register(ApplicationListenerRegistration::new(
        "audit",
        Arc::new(AuditListener {
            state: Arc::clone(state),
        }),
    ));

    // Channel: the event is queued, and the worker delivers one at a time.
    multicaster.register(
        ApplicationListenerRegistration::new(
            "notifier",
            Arc::new(NotificationListener {
                state: Arc::clone(state),
            }),
        )
        .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(
            NOTIFICATION_CAPACITY,
        ))),
    );

    // Channel with a queue of its own: a listener that is slower, or that is
    // broken, does not hold back the listeners of the other queues.
    multicaster.register(
        ApplicationListenerRegistration::new(
            "alert",
            Arc::new(AlertListener {
                state: Arc::clone(state),
            }),
        )
        .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(
            ALERT_CAPACITY,
        ))),
    );
}

/// Contributes the state the listeners of the example write.
///
/// A singleton provider is registered before the context refreshes, and the
/// instance is created when it is first resolved. That is what lets the
/// multicaster below depend on it: the context creates the multicaster while it
/// refreshes, and the multicaster resolves the state it needs.
#[singleton(name = "eventsState")]
fn events_state() -> Arc<EventsState> {
    Arc::new(EventsState::default())
}

/// Contributes the multicaster of the application and registers the listeners of
/// the example on it.
///
/// The context resolves a multicaster the application declares while it
/// refreshes, so every event of the application goes through this one — the
/// publisher of the context hands the events to it — and the routes of the
/// example read the statistics of its queues.
///
/// The argument is resolved when the multicaster is created, not when it is
/// declared: a provider is created on demand, so the order the providers are
/// declared in does not matter.
#[singleton(name = "applicationEventMulticaster")]
fn multicaster(events_state: Arc<EventsState>) -> Arc<dyn ApplicationEventMulticaster> {
    let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
    register_listeners(&multicaster, &events_state);
    let _ = events_state.multicaster.set(Arc::clone(&multicaster));

    multicaster
}

/// Publishes an order and reports what each kind of listener did with it.
#[post_mapping(path = "/events/orders/{id}/{amount}")]
async fn place_order(
    Path((id, amount)): Path<(String, u64)>,
    FindSingleton(events_state): FindSingleton<Arc<EventsState>>,
) -> impl IntoResponse {
    let before = events_state.audit().len();
    events_state.publish(&id, amount);
    let after = events_state.audit().len();

    format!(
        "published order {id}\n\
         audit: {before} -> {after} (the inline listener ran before the call returned)\n\
         notifications: {} (the channel listener has not delivered it yet)",
        events_state.notifications().len()
    )
}

/// Publishes a burst of orders, which fills the queues of the slow listeners.
#[post_mapping(path = "/events/burst/{count}/{amount}")]
async fn publish_burst(
    Path((count, amount)): Path<(usize, u64)>,
    FindSingleton(events_state): FindSingleton<Arc<EventsState>>,
) -> impl IntoResponse {
    for index in 0..count {
        events_state.publish(&format!("burst-{index}"), amount);
    }

    format!(
        "published {count} orders\n\
         audit: {}\n\
         notifier: {}\n\
         alert: {}",
        events_state.audit().len(),
        events_state
            .statistics("notifier")
            .unwrap_or_else(|| String::from("no queue")),
        events_state
            .statistics("alert")
            .unwrap_or_else(|| String::from("no queue")),
    )
}

/// Reports what the listeners of the example have done so far.
#[get_mapping(path = "/events")]
async fn report(
    FindSingleton(events_state): FindSingleton<Arc<EventsState>>,
) -> impl IntoResponse {
    format!(
        "audit: {:?}\n\
         notifications: {:?}\n\
         alerts: {}\n\
         notifier: {}\n\
         alert: {}",
        events_state.audit(),
        events_state.notifications(),
        events_state.alerts(),
        events_state
            .statistics("notifier")
            .unwrap_or_else(|| String::from("no queue")),
        events_state
            .statistics("alert")
            .unwrap_or_else(|| String::from("no queue")),
    )
}

#[tokio::main]
async fn main() {
    NextWebApplication::<EventsApplication>::default().run().await
}


#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web::{ConfigurableApplicationContext, context::DefaultApplicationContext};
    use next_web_context::{
        APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
        APPLICATION_EVENT_PUBLISHER_SINGLETON_NAME, ApplicationContextExt, ApplicationEventPublisher,
        event::{ApplicationEventMulticaster, DefaultApplicationEventMulticaster},
    };

    use super::{EventsState, OrderPlaced, register_listeners};

    /// Returns a multicaster with the listeners of the example registered on it.
    fn runtime() -> (Arc<EventsState>, Arc<DefaultApplicationEventMulticaster>) {
        let state = Arc::new(EventsState::default());
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        register_listeners(&multicaster, &state);
        let _ = state.multicaster.set(Arc::clone(&multicaster));

        (state, multicaster)
    }

    /// Publishes an order through the multicaster of the example.
    fn publish(multicaster: &DefaultApplicationEventMulticaster, id: &str, amount: u64) {
        multicaster
            .multicast_event(Box::new(OrderPlaced {
                id: id.to_owned(),
                amount,
            }))
            .expect("the order is published");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_inline_listener_has_run_when_the_publishing_call_returns() {
        let (state, multicaster) = runtime();

        publish(&multicaster, "order-1", 10);

        assert_eq!(
            state.audit(),
            vec![String::from("order order-1 at 10 cents")],
            "the audit listener wrote before the publishing call returned"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_channel_listener_is_delivered_after_the_publishing_call_returns() {
        let (state, multicaster) = runtime();

        publish(&multicaster, "order-2", 10);
        assert!(
            state.notifications().is_empty(),
            "the notifier had not run when the publishing call returned"
        );

        multicaster.flush_async().await;

        assert_eq!(
            state.notifications(),
            vec![String::from("notified order-2")]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_burst_fills_the_queue_and_the_refused_events_are_counted() {
        let (state, multicaster) = runtime();

        for index in 0..100 {
            publish(&multicaster, &format!("burst-{index}"), 10);
        }
        let statistics = multicaster
            .channel_statistics("notifier")
            .expect("the notifier has a queue of its own");

        assert_eq!(
            statistics.sent + statistics.rejected,
            100,
            "every event was either queued or refused, it was: {statistics}"
        );
        assert!(
            statistics.rejected > 0,
            "a burst that is faster than the listener fills the queue, it was: {statistics}"
        );
        multicaster.close_async().await;
        assert_eq!(
            state.audit().len(),
            100,
            "the inline listener ran for every event of the burst"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_alert_that_fails_does_not_stop_its_queue() {
        let (state, multicaster) = runtime();

        publish(&multicaster, "big-order", 5000);
        publish(&multicaster, "small-order", 10);
        multicaster.flush_async().await;

        assert_eq!(
            state.alerts(),
            2,
            "the alert listener ran for both orders"
        );
        assert_eq!(
            state.notifications(),
            vec![
                String::from("notified big-order"),
                String::from("notified small-order")
            ],
            "the notifier delivered both orders although the alert listener failed"
        );
        let alert = multicaster
            .channel_statistics("alert")
            .expect("the alert listener has a queue of its own");
        assert_eq!(alert.failed, 1, "the failure of the listener was counted");
        multicaster.close_async().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_close_delivers_the_events_that_are_queued() {
        let (state, multicaster) = runtime();

        for index in 0..5 {
            publish(&multicaster, &format!("order-{index}"), 10);
        }
        multicaster.close_async().await;

        assert_eq!(
            state.notifications().len(),
            5,
            "the shutdown drained the queue before the workers stopped"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_publisher_of_the_context_reaches_the_listeners_of_the_example() {
        let (state, multicaster) = runtime();
        let mut context = DefaultApplicationContext::default();
        // The context uses a multicaster the application installed, so the
        // listeners of the example receive the events the application publishes.
        context.insert_singleton_with_name::<Arc<dyn ApplicationEventMulticaster>>(
            multicaster,
            APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
        );
        context.refresh().expect("the context refreshes");

        // The publisher of an application is its context, and it reaches the
        // listeners the application registered.
        let publisher = context
            .get_singleton_option_with_name::<Arc<dyn ApplicationEventPublisher>>(
                APPLICATION_EVENT_PUBLISHER_SINGLETON_NAME,
            )
            .cloned()
            .expect("the context installs the publisher of the application");
        publisher
            .publish_event(Box::new(OrderPlaced {
                id: String::from("order-from-the-context"),
                amount: 10,
            }))
            .expect("the order is published");
        state
            .multicaster()
            .expect("the state holds the multicaster")
            .flush_async()
            .await;

        assert_eq!(
            state.audit(),
            vec![String::from("order order-from-the-context at 10 cents")],
            "the inline listener received the event the context published"
        );
        assert_eq!(
            state.notifications(),
            vec![String::from("notified order-from-the-context")]
        );
        context.close().expect("the context closes");
    }
}
