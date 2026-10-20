//! The application event example: one event, two listeners, two ways to deliver
//! it.
//!
//! The routes publish an event through the publisher of the application, and the
//! multicaster of the context hands it to the listeners that support it:
//!
//! - `AuditListener` is dispatched **inline**: it runs as part of the call that
//!   publishes the event, so what it writes is there when the call returns;
//! - `NotifierListener` is dispatched over a **channel**: the publishing call only
//!   queues the event, and the listener runs on a worker of its own, one event
//!   after the other. It waits 50 milliseconds, like a mail or a webhook would.
//!
//! Try it with:
//!
//! ```text
//! cargo run -p next-web-examples --bin application_event
//! curl -X POST http://127.0.0.1:11000/app/events/orders/order-1
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

use next_web::{
    Application, NextWebApplication,
    core::BoxFuture,
    extract::Path,
    macros::bind::{get_mapping, post_mapping, singleton},
};
use next_web_context::{
    ApplicationEvent, ApplicationEventPublisher, ApplicationEventPublisherExt, ApplicationListener,
    event::{
        ApplicationEventMulticaster, ApplicationListenerRegistration, ChannelDispatch,
        DefaultApplicationEventMulticaster, ListenerDispatch,
    },
};

#[derive(Default)]
pub struct EventsApplication;

impl Application for EventsApplication {}

/// The event of the example.
///
/// The derive writes the four methods the event system asks for: the type of the
/// event and the type of its source are the type of the value, and the source of
/// the event is the value itself.
#[derive(Clone, Debug, next_web::macros::event::ApplicationEvent)]
pub struct OrderPlaced {
    /// The identifier of the order.
    pub id: String,
}

/// What the listeners of the example received.
///
/// The listeners write into it and the routes read it, which is how a route shows
/// what each kind of dispatch did with the event it published.
#[derive(Default)]
pub struct Received {
    /// What the inline listener wrote.
    inline: Mutex<Vec<String>>,
    /// What the channel dispatched listener wrote.
    channel: Mutex<Vec<String>>,
    /// The number of deliveries that were in flight at the same time.
    in_flight: AtomicUsize,
    /// The multicaster the listeners are registered on.
    multicaster: OnceLock<Arc<DefaultApplicationEventMulticaster>>,
}

impl Received {
    /// Returns what the inline listener wrote.
    fn inline(&self) -> Vec<String> {
        self.inline
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Returns what the channel dispatched listener wrote.
    fn channel(&self) -> Vec<String> {
        self.channel
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Returns the number of deliveries that are in flight.
    fn in_flight(&self) -> usize {
        self.in_flight.load(Ordering::SeqCst)
    }

    /// Returns the multicaster the listeners of the example are registered on.
    fn multicaster(&self) -> Option<Arc<DefaultApplicationEventMulticaster>> {
        self.multicaster.get().cloned()
    }
}

/// Writes the order into the audit, as part of the publishing call.
struct AuditListener {
    received: Arc<Received>,
}

impl ApplicationListener<Arc<dyn ApplicationEvent>> for AuditListener {
    fn on_application_event<'a>(&'a self, event: Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if let Some(order) = (event.as_ref() as &dyn Any).downcast_ref::<OrderPlaced>() {
                self.received
                    .inline
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(format!("audited {}", order.id));
            }
        })
    }
}

/// Notifies the order, slowly, on the worker of its queue.
struct NotifierListener {
    received: Arc<Received>,
}

impl ApplicationListener<Arc<dyn ApplicationEvent>> for NotifierListener {
    fn on_application_event<'a>(&'a self, event: Arc<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let Some(order) = (event.as_ref() as &dyn Any).downcast_ref::<OrderPlaced>() else {
                return;
            };
            let id = order.id.clone();

            self.received.in_flight.fetch_add(1, Ordering::SeqCst);
            // A mail, a webhook or a call to another service would be here.
            tokio::time::sleep(Duration::from_millis(50)).await;

            self.received
                .channel
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(format!("notified {id}"));
            self.received.in_flight.fetch_sub(1, Ordering::SeqCst);
        })
    }
}

/// Registers the two listeners on the multicaster.
///
/// This is the whole difference between the two kinds: the registration says how
/// the event reaches the listener.
///
/// # Arguments
///
/// * `multicaster` - The multicaster the listeners are registered on.
/// * `received` - The state the listeners write.
fn register_listeners(multicaster: &DefaultApplicationEventMulticaster, received: &Arc<Received>) {
    // Inline: runs as part of the publishing call.
    multicaster.register(ApplicationListenerRegistration::new(
        "audit",
        Arc::new(AuditListener {
            received: Arc::clone(received),
        }),
    ));

    // Channel: the event is queued, and the worker delivers one at a time.
    multicaster.register(
        ApplicationListenerRegistration::new(
            "notifier",
            Arc::new(NotifierListener {
                received: Arc::clone(received),
            }),
        )
        .with_dispatch(ListenerDispatch::Channel(ChannelDispatch::with_capacity(
            16,
        ))),
    );
}

/// Contributes the state the listeners of the example write.
#[singleton(name = "received")]
fn received() -> Arc<Received> {
    Arc::new(Received::default())
}

/// Contributes the multicaster of the application and registers the listeners.
///
/// The context resolves a multicaster the application declares while it refreshes,
/// so the events the publisher of the application hands over reach the listeners of
/// the example.
#[singleton(name = "applicationEventMulticaster")]
fn multicaster(received: Arc<Received>) -> Arc<dyn ApplicationEventMulticaster> {
    let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
    register_listeners(&multicaster, &received);
    let _ = received.multicaster.set(Arc::clone(&multicaster));

    multicaster
}

/// Publishes an order and reports what each kind of listener did with it.
#[post_mapping(path = "/events/orders/{id}")]
async fn place_order(
    Path(id): Path<String>,
    received: FindSingleton<Arc<Received>>,
    publisher: FindSingleton<Arc<dyn ApplicationEventPublisher>>,
) -> String {
    publisher
        .publish(OrderPlaced { id })
        .expect("the event is published");

    format!(
        "inline: {:?}\nchannel: {:?}",
        received.inline(),
        received.channel()
    )
}

/// Reports what the listeners received so far, and what the channel queue holds.
#[get_mapping(path = "/events")]
async fn list(received: FindSingleton<Arc<Received>>) -> String {
    let statistics = received
        .multicaster()
        .and_then(|multicaster| multicaster.channel_statistics("notifier"));

    format!(
        "inline: {:?}\nchannel: {:?}\nin flight: {}\nnotifier: {}",
        received.inline(),
        received.channel(),
        received.in_flight(),
        statistics
            .map(|statistics| statistics.to_string())
            .unwrap_or_else(|| String::from("no queue")),
    )
}

#[tokio::main]
async fn main() {
    NextWebApplication::<EventsApplication>::default()
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web::ConfigurableApplicationContext;
    use next_web_context::event::{
        ApplicationEventMulticaster, DefaultApplicationEventMulticaster,
    };

    use super::{OrderPlaced, Received, register_listeners};

    /// Returns a multicaster with the listeners of the example on it.
    fn runtime() -> (Arc<Received>, Arc<DefaultApplicationEventMulticaster>) {
        let received = Arc::new(Received::default());
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        register_listeners(&multicaster, &received);
        let _ = received.multicaster.set(Arc::clone(&multicaster));

        (received, multicaster)
    }

    /// Publishes an order the way the route does.
    fn publish(multicaster: &DefaultApplicationEventMulticaster, id: &str) {
        multicaster
            .multicast_event(Arc::new(OrderPlaced { id: id.to_owned() }))
            .expect("the order is published");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_inline_listener_runs_before_the_publishing_call_returns() {
        let (received, multicaster) = runtime();

        publish(&multicaster, "order-1");

        assert_eq!(received.inline(), vec![String::from("audited order-1")]);
        assert!(
            received.channel().is_empty(),
            "the channel dispatched listener does not run in the publishing call"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_channel_listener_runs_after_the_publishing_call_returns() {
        let (received, multicaster) = runtime();

        publish(&multicaster, "order-1");
        multicaster.flush_async().await;

        assert_eq!(received.channel(), vec![String::from("notified order-1")]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_channel_listener_delivers_one_event_at_a_time() {
        let (received, multicaster) = runtime();

        for index in 0..5 {
            publish(&multicaster, &format!("order-{index}"));
        }
        let in_flight = received.in_flight();
        multicaster.close_async().await;

        assert_eq!(in_flight, 1, "the queue delivers one event at a time");
        assert_eq!(
            received.channel().len(),
            5,
            "the shutdown drained the queue before the worker stopped"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_publisher_of_the_context_reaches_the_listeners() {
        use next_web::context::DefaultApplicationContext;
        use next_web_context::{
            APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME, ApplicationContextExt,
            ApplicationEventPublisher,
        };

        let (received, multicaster) = runtime();
        let mut context = DefaultApplicationContext::default();
        context.insert_singleton_with_name::<Arc<dyn ApplicationEventMulticaster>>(
            multicaster,
            APPLICATION_EVENT_MULTICASTER_SINGLETON_NAME,
        );
        context.refresh().expect("the context refreshes");

        let publisher = context
            .get_singleton_option_with_name::<Arc<dyn ApplicationEventPublisher>>(
                next_web_context::APPLICATION_EVENT_PUBLISHER_SINGLETON_NAME,
            )
            .cloned()
            .expect("the context installs the publisher of the application");
        publisher
            .publish_event(Arc::new(OrderPlaced {
                id: String::from("order-1"),
            }))
            .expect("the order is published");
        received
            .multicaster()
            .expect("the state holds the multicaster")
            .flush_async()
            .await;

        assert_eq!(received.inline(), vec![String::from("audited order-1")]);
        assert_eq!(received.channel(), vec![String::from("notified order-1")]);
        context.close().expect("the context closes");
    }
}
