use std::{
    any::TypeId,
    fmt::{self, Debug},
};

use crate::{
    ApplicationEvent, ApplicationListener,
    event::{ErasedApplicationListener, ListenerDispatch, TypedApplicationListener},
};

/// A listener together with the metadata the multicaster dispatches with.
///
/// The metadata is what makes an event reach only the listeners that are meant
/// to see it:
///
/// - the `order` decides when a listener runs relative to the others,
/// - the [`TypeId`] of the event type, when there is one, filters the events
///   the listener is called for,
/// - the [`ListenerDispatch`] decides whether the listener runs as part of the
///   publishing call or on a worker of its own.
///
/// A registration without an event type receives every event, which is what a
/// listener of `Box<dyn ApplicationEvent>` asks for.
#[derive(Clone)]
pub struct ApplicationListenerRegistration {
    id: String,
    order: i32,
    event_type: Option<TypeId>,
    dispatch: ListenerDispatch,
    listener: ErasedApplicationListener,
}

impl ApplicationListenerRegistration {
    /// Creates a registration of `listener` under `id`.
    ///
    /// The listener receives every event, and it runs in the order of its
    /// registration until [`Self::with_order`] is used. It is dispatched inline
    /// until [`Self::with_dispatch`] is used.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier the listener is registered under. Registering
    ///   another listener under the same identifier replaces this one.
    /// * `listener` - The type erased listener.
    pub fn new(id: impl Into<String>, listener: ErasedApplicationListener) -> Self {
        Self {
            id: id.into(),
            order: 0,
            event_type: None,
            dispatch: ListenerDispatch::default(),
            listener,
        }
    }

    /// Creates a registration of a listener of one concrete event type.
    ///
    /// The listener is called for the events of `E` only, so it does not have
    /// to inspect the events it receives. The event type also filters the
    /// events before the listener is called, which keeps an event from reaching
    /// a listener that cannot handle it.
    ///
    /// # Arguments
    ///
    /// * `id` - The identifier the listener is registered under.
    /// * `listener` - The listener of the concrete event type.
    pub fn typed<L, E>(id: impl Into<String>, listener: L) -> Self
    where
        L: ApplicationListener<E> + 'static,
        E: ApplicationEvent + 'static,
    {
        let listener = TypedApplicationListener::<L, E>::new(listener);
        Self::new(id, std::sync::Arc::new(listener)).with_event_type::<E>()
    }

    /// Sets the order the listener runs in.
    ///
    /// Listeners run in ascending order; listeners of the same order run in the
    /// order they were registered in.
    ///
    /// # Arguments
    ///
    /// * `order` - The order of this listener.
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// Restricts the listener to the events of type `E`.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The event type the listener handles.
    pub fn with_event_type<E>(mut self) -> Self
    where
        E: ApplicationEvent + 'static,
    {
        self.event_type = Some(TypeId::of::<E>());
        self
    }

    /// Restricts the listener to the events of the given type.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The [`TypeId`] of the event type the listener handles.
    pub fn with_event_type_id(mut self, event_type: TypeId) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Sets how the events of the listener reach it.
    ///
    /// A listener is dispatched inline by default: it runs as part of the call
    /// that publishes the event. [`ListenerDispatch::Channel`] queues the event
    /// for a worker of the listener instead, which keeps a slow listener from
    /// slowing down the request that published the event. See
    /// [`ListenerDispatch`] for what each kind guarantees.
    ///
    /// # Arguments
    ///
    /// * `dispatch` - How the listener is dispatched.
    pub fn with_dispatch(mut self, dispatch: ListenerDispatch) -> Self {
        self.dispatch = dispatch;
        self
    }

    /// Returns how the events of the listener reach it.
    pub fn dispatch(&self) -> ListenerDispatch {
        self.dispatch
    }

    /// Returns the identifier the listener is registered under.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the order the listener runs in.
    pub fn order(&self) -> i32 {
        self.order
    }

    /// Returns the event type the listener is restricted to, when it is.
    pub fn event_type(&self) -> Option<TypeId> {
        self.event_type
    }

    /// Returns whether the listener is called for an event of `event_type`.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The type of the event that is published.
    pub fn supports(&self, event_type: TypeId) -> bool {
        self.event_type
            .map(|supported| supported == event_type)
            .unwrap_or(true)
    }

    /// Returns the type erased listener.
    pub fn listener(&self) -> &ErasedApplicationListener {
        &self.listener
    }
}

impl Debug for ApplicationListenerRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApplicationListenerRegistration")
            .field("id", &self.id)
            .field("order", &self.order)
            .field("event_type", &self.event_type)
            .field("dispatch", &self.dispatch)
            .finish_non_exhaustive()
    }
}
