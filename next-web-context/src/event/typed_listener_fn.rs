//! The listener of one event type, written as a closure.

use std::{any::TypeId, future::Future, marker::PhantomData, sync::Arc};

use crate::{
    ApplicationEvent, ApplicationListener, BoxFuture,
    event::{ErasedApplicationListener, downcast_event},
};

/// Adapts a function of one event type to the type erased listener the
/// multicaster stores.
///
/// A listener that handles one type of event does not have to be a type of its own:
/// the adapter of the event type calls the handler with the shared event when the
/// event is of the type it was registered for, and ignores the events of the other
/// types. The handler therefore sees no downcast and no case it has to ignore.
///
/// The handler receives the event that was published, shared: reading the fields of
/// the event, and holding it across an `await`, costs nothing. An event that is
/// dispatched over a channel is the one the publisher created, so an event is not
/// copied for the listener.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
///
/// use next_web_context::{
///     ApplicationEvent, ApplicationListener,
///     event::typed_listener_fn,
/// };
///
/// #[derive(Clone)]
/// struct OrderPlaced {
///     id: String,
/// }
///
/// impl ApplicationEvent for OrderPlaced {
///     fn source(&self) -> &dyn std::any::Any {
///         self
///     }
/// }
///
/// let listener = typed_listener_fn::<OrderPlaced, _, _>(|order| async move {
///     println!("the order {} was placed", order.id);
/// });
/// multicaster.add_application_listener(String::from("notifier"), listener);
/// ```
pub struct TypedListenerFn<E, F> {
    handler: F,
    event: PhantomData<fn() -> E>,
}

impl<E, F> TypedListenerFn<E, F> {
    /// Wraps the handler, which is called for the events of type `E`.
    ///
    /// # Arguments
    ///
    /// * `handler` - The handler of the event.
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            event: PhantomData,
        }
    }
}

impl<E, F> std::fmt::Debug for TypedListenerFn<E, F>
where
    E: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedListenerFn")
            .field("event_type", &TypeId::of::<E>())
            .finish_non_exhaustive()
    }
}

impl<E, F, Fut> ApplicationListener<Arc<dyn ApplicationEvent>> for TypedListenerFn<E, F>
where
    E: ApplicationEvent + 'static,
    F: Fn(Arc<E>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    fn on_application_event<'a>(
        &'a self,
        event: &'a Arc<dyn ApplicationEvent>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            match downcast_event::<E>(Arc::clone(event)) {
                // The event is shared, so the handler reads the one value the
                // application published instead of a copy of it.
                Ok(event) => (self.handler)(event).await,
                Err(event) => tracing::debug!(
                    expected = ?TypeId::of::<E>(),
                    received = ?event.event_type(),
                    "the listener ignored an event of another type"
                ),
            }
        })
    }
}

/// Wraps a function of one event type into the listener the multicaster accepts.
///
/// # Arguments
///
/// * `handler` - The handler of the event, which receives the shared event.
pub fn typed_listener_fn<E, F, Fut>(handler: F) -> ErasedApplicationListener
where
    E: ApplicationEvent + 'static,
    F: Fn(Arc<E>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    Arc::new(TypedListenerFn::<E, F>::new(handler))
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
        ApplicationEvent,
        event::{ApplicationEventMulticaster, DefaultApplicationEventMulticaster, typed_listener_fn},
    };

    /// The event of the tests, which is not cloneable: a listener of it reads the
    /// value that was published.
    #[derive(Debug)]
    struct Ping {
        /// The identifier of the event of the test.
        id: u32,
    }

    impl ApplicationEvent for Ping {
        fn source(&self) -> &dyn Any {
            self
        }
    }

    /// Another event, so that a listener of `Ping` has an event it is not called
    /// for.
    #[derive(Debug)]
    struct Pong;

    impl ApplicationEvent for Pong {
        fn source(&self) -> &dyn Any {
            self
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_handler_is_called_with_the_event_it_handles_and_no_other() {
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        let received = Arc::new(Mutex::new(Vec::new()));
        let listener = {
            let received = Arc::clone(&received);

            typed_listener_fn::<Ping, _, _>(move |ping| {
                let received = Arc::clone(&received);

                async move {
                    received
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(ping.id);
                }
            })
        };

        multicaster.add_application_listener(String::from("ping"), listener);
        multicaster
            .multicast_event(Arc::new(Ping { id: 1 }))
            .expect("the ping is published");
        multicaster
            .multicast_event(Arc::new(Pong))
            .expect("the pong is published");

        assert_eq!(
            *received
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![1],
            "the handler was called for its event only"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_event_of_the_handler_is_not_copied_for_it() {
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        let addresses = Arc::new(Mutex::new(Vec::new()));
        let published = Arc::new(Ping { id: 2 });
        let listener = {
            let addresses = Arc::clone(&addresses);

            typed_listener_fn::<Ping, _, _>(move |ping| {
                let addresses = Arc::clone(&addresses);

                async move {
                    addresses
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(Arc::as_ptr(&ping) as *const () as usize);
                }
            })
        };

        multicaster.add_application_listener(String::from("ping"), listener);
        multicaster
            .multicast_event(Arc::clone(&published) as Arc<dyn ApplicationEvent>)
            .expect("the ping is published");

        assert_eq!(
            *addresses
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
            vec![Arc::as_ptr(&published) as *const () as usize],
            "the handler received the value that was published"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_handler_that_panics_does_not_stop_the_other_listeners() {
        let multicaster = Arc::new(DefaultApplicationEventMulticaster::new_boxed());
        let calls = Arc::new(AtomicUsize::new(0));
        let panicking = typed_listener_fn::<Ping, _, _>(|_ping| async move {
            panic!("the listener of the test panics");
        });
        let counting = {
            let calls = Arc::clone(&calls);

            typed_listener_fn::<Ping, _, _>(move |_ping| {
                let calls = Arc::clone(&calls);

                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                }
            })
        };

        multicaster.add_application_listener(String::from("panicking"), panicking);
        multicaster.add_application_listener(String::from("counting"), counting);
        let _ = multicaster.multicast_event(Arc::new(Ping { id: 3 }));

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "the listener of the event still ran"
        );
    }
}
