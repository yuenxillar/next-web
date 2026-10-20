use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc,
};

use crate::{ApplicationEvent, ApplicationListener, BoxFuture};

/// Adapts a listener of one concrete event type to the type erased listener the
/// multicaster stores.
///
/// Every listener is registered as an
/// [`ApplicationListener<Arc<dyn ApplicationEvent>>`](ApplicationListener),
/// because a multicaster publishes the events of an application without knowing
/// their types. A listener of a concrete event type therefore needs an adapter
/// that turns the erased event back into the event the listener handles. The
/// adapter ignores an event of another type instead of failing, so it is safe to
/// register it without an event type filter.
///
/// # Example
///
/// ```ignore
/// let listener = TypedApplicationListener::<MyListener, MyEvent>::new(MyListener);
/// multicaster.register(ApplicationListenerRegistration::new("myListener", Arc::new(listener)));
/// ```
pub struct TypedApplicationListener<L, E> {
    listener: L,
    event: PhantomData<fn() -> E>,
}

impl<L, E> TypedApplicationListener<L, E>
where
    E: 'static,
{
    /// Wraps `listener` so that it can be registered with a multicaster.
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener of the concrete event type.
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            event: PhantomData,
        }
    }

    /// Returns the wrapped listener.
    pub fn inner(&self) -> &L {
        &self.listener
    }

    /// Returns the event type the wrapped listener handles.
    pub fn event_type() -> TypeId {
        TypeId::of::<E>()
    }
}

impl<L, E> ApplicationListener<Arc<dyn ApplicationEvent>> for TypedApplicationListener<L, E>
where
    L: ApplicationListener<E> + 'static,
    E: ApplicationEvent + 'static,
{
    fn on_application_event<'a>(
        &'a self,
        event: &'a Arc<dyn ApplicationEvent>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            // The downcast happens here and nowhere else: the listener of the type
            // is handed the event it declared, borrowed from the value the
            // application published.
            match downcast_event::<E>(Arc::clone(event)) {
                Ok(event) => self.listener.on_application_event(event.as_ref()).await,
                Err(event) => {
                    tracing::debug!(
                        expected = ?TypeId::of::<E>(),
                        received = ?event.event_type(),
                        "application listener ignored an event of another type"
                    );
                }
            }
        })
    }
}

impl<L, E> std::fmt::Debug for TypedApplicationListener<L, E>
where
    E: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedApplicationListener")
            .field("event_type", &TypeId::of::<E>())
            .finish_non_exhaustive()
    }
}

/// Wraps the listener in the adapter the multicaster accepts.
///
/// # Arguments
///
/// * `listener` - The listener of the concrete event type.
pub fn typed_listener<L, E>(listener: L) -> Arc<TypedApplicationListener<L, E>>
where
    L: ApplicationListener<E> + 'static,
    E: ApplicationEvent + 'static,
{
    Arc::new(TypedApplicationListener::new(listener))
}

/// Turns a shared event back into the event of type `E`.
///
/// The event is handed back unchanged when it is of another type, so that the
/// caller can report the mismatch without losing the event.
///
/// # Arguments
///
/// * `event` - The shared event to downcast.
pub fn downcast_event<E>(event: Arc<dyn ApplicationEvent>) -> Result<Arc<E>, Arc<dyn ApplicationEvent>>
where
    E: ApplicationEvent + 'static,
{
    // `type_id` is the type id of the concrete event, while `event_type` is the
    // type an event reports, which an event may define in another way.
    if (*event).type_id() != TypeId::of::<E>() {
        return Err(event);
    }

    // The event is an `E`, so the conversion goes through `Any`: a shared event
    // cannot be downcast to a type that is only known at runtime otherwise.
    let event: Arc<dyn Any + Send + Sync> = event;

    Ok(event
        .downcast::<E>()
        .unwrap_or_else(|_| unreachable!("the event is an instance of the type it was checked against")))
}

#[cfg(test)]
mod tests {
    use std::{
        any::Any,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use crate::{
        ApplicationEvent, ApplicationListener, BoxFuture,
        event::{
            ApplicationEventMulticaster, ApplicationListenerRegistration,
            DefaultApplicationEventMulticaster,
        },
    };

    /// The event of the tests, which is not cloneable: the listener of it is handed
    /// the value that was published.
    struct Ping {
        /// The identifier of the event of the test.
        id: u32,
    }

    impl ApplicationEvent for Ping {
        fn source(&self) -> &dyn Any {
            self
        }
    }

    /// Another event, so that the listener of `Ping` has an event it is never
    /// called for.
    struct Pong;

    impl ApplicationEvent for Pong {
        fn source(&self) -> &dyn Any {
            self
        }
    }

    /// The listener of one event type, written as a type of its own.
    struct PingListener {
        received: Arc<Mutex<Vec<u32>>>,
    }

    impl ApplicationListener<Ping> for PingListener {
        fn on_application_event<'a>(&'a self, ping: &'a Ping) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.received
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .push(ping.id);
            })
        }
    }

    /// A listener of the shared event that counts what it receives.
    #[derive(Default)]
    struct Counter {
        received: AtomicUsize,
    }

    impl ApplicationListener<Arc<dyn ApplicationEvent>> for Counter {
        fn on_application_event<'a>(
            &'a self,
            _event: &'a Arc<dyn ApplicationEvent>,
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move {
                self.received.fetch_add(1, Ordering::SeqCst);
            })
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_listener_of_one_event_type_reads_the_event_it_was_registered_for() {
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        let received = Arc::new(Mutex::new(Vec::new()));
        let listener = PingListener {
            received: Arc::clone(&received),
        };

        multicaster.register(ApplicationListenerRegistration::typed("ping", listener));
        multicaster
            .multicast_event(Arc::new(Ping { id: 4 }))
            .expect("the ping is published");
        multicaster
            .multicast_event(Arc::new(Pong))
            .expect("the pong is published");

        assert_eq!(
            *received.lock().unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![4],
            "the listener was called for its event only, and the event was not copied for it"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_listener_of_the_shared_event_reads_the_one_value_published() {
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        let counter = Arc::new(Counter::default());
        let listener: Arc<dyn ApplicationListener<Arc<dyn ApplicationEvent>>> = counter.clone();

        multicaster.add_application_listener(String::from("counter"), listener);
        multicaster
            .multicast_event(Arc::new(Ping { id: 5 }))
            .expect("the ping is published");

        assert_eq!(counter.received.load(Ordering::SeqCst), 1);
    }
}