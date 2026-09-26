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
/// [`ApplicationListener<Box<dyn ApplicationEvent>>`](ApplicationListener),
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

impl<L, E> ApplicationListener<Box<dyn ApplicationEvent>> for TypedApplicationListener<L, E>
where
    L: ApplicationListener<E> + 'static,
    E: ApplicationEvent + 'static,
{
    fn on_application_event<'a>(&'a self, event: Box<dyn ApplicationEvent>) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            match downcast_event::<E>(event) {
                Ok(event) => self.listener.on_application_event(*event).await,
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

/// Turns an erased event back into the event of type `E`.
///
/// The event is handed back unchanged when it is of another type, so that the
/// caller can report the mismatch without losing the event.
///
/// # Arguments
///
/// * `event` - The erased event to downcast.
pub fn downcast_event<E>(event: Box<dyn ApplicationEvent>) -> Result<Box<E>, Box<dyn ApplicationEvent>>
where
    E: ApplicationEvent + 'static,
{
    // `type_id` is the type id of the concrete event, while `event_type` is the
    // type an event reports, which an event may define in another way.
    if (*event).type_id() != TypeId::of::<E>() {
        return Err(event);
    }

    // The event is an `E`, so the conversion goes through `Any`; a trait object
    // cannot be downcast to a type that is only known at runtime otherwise.
    let event: Box<dyn Any + Send + Sync> = event;

    Ok(event
        .downcast::<E>()
        .unwrap_or_else(|_| unreachable!("the event is an instance of the type it was checked against")))
}
