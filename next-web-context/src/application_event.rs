use std::{
    any::{Any, TypeId},
    sync::Arc,
};

/// The event of an application.
///
/// An event is a value the event system publishes and hands to the listeners that
/// support it. It is published once and shared by every listener as an
/// `Arc<dyn ApplicationEvent>`, so an event does not have to be cloneable and a
/// large event is not copied for each listener. The trait asks for the type of the
/// event and for the source of it so that a listener is called only for the events
/// it handles; both have a default that is right for the usual event, which is a
/// value that is its own source.
///
/// # Requirements
///
/// - `Send + Sync`: an event is shared by the listeners of the threads that
///   dispatch it;
/// - `Any`: an event is downcast to its type by a listener that is adapted to one
///   type of event.
///
/// An event does not have to be `Clone`: the event system publishes one value and
/// hands the same `Arc` to every listener. A listener that is adapted to one type
/// of event does clone the event it handles, because the trait of a listener takes
/// the event it handles by value, and such an event has to be `Clone`.
///
/// # Example
///
/// ```rust
/// use std::any::Any;
///
/// use next_web_context::ApplicationEvent;
///
/// #[derive(Clone)]
/// struct UserLoggedIn {
///     user_id: u64,
/// }
///
/// impl ApplicationEvent for UserLoggedIn {
///     fn source(&self) -> &dyn Any {
///         self
///     }
/// }
/// ```
///
/// The three other methods are the defaults of the trait: the event reports its
/// own type, the source of the event is the event itself, and the event carries no
/// timestamp. An event that wraps the object that published it overrides
/// [`ApplicationEvent::source`] and [`ApplicationEvent::source_type`], and an event
/// that is stamped overrides [`ApplicationEvent::timestamp`].
///
/// The implementation is what `#[derive(ApplicationEvent)]` of `next-web-macros`
/// writes.
pub trait ApplicationEvent
where
    Self: Send + Sync,
    Self: Any,
{
    /// Returns the timestamp when this event occurred, in seconds since the Unix
    /// epoch.
    ///
    /// The default is `0`, which means that the event carries no timestamp; an
    /// event that is stamped overrides it. The value is the one
    /// [`EventAttributes`] stamps, and an application that needs the age of an
    /// event reads it from there.
    fn timestamp(&self) -> u64 {
        0
    }

    /// Returns the source of the event as a `dyn Any` reference.
    ///
    /// The source is the object that published the event, and it is type erased,
    /// so a listener that reads it downcasts it. An event that is its own source
    /// (the common case, and what `#[derive(ApplicationEvent)]` generates) answers
    /// `self`.
    fn source(&self) -> &dyn Any;

    /// Returns the type of the event.
    ///
    /// The default is the type of the value the call is made on, which is what
    /// listeners are filtered by and what the downcast of an adapted listener
    /// checks. An implementation must not override it: a type the method reports
    /// that is not the type of the event makes the two disagree, and an event
    /// that passes the filter of a listener is then ignored by it.
    fn event_type(&self) -> TypeId {
        Any::type_id(self)
    }

    /// Returns the type of the source of the event.
    ///
    /// The default is the type of the event, which is what an event that is its
    /// own source answers. An event that wraps the object that published it
    /// reports the type of that object, so a listener that filters by source type
    /// reads it.
    fn source_type(&self) -> TypeId {
        Any::type_id(self)
    }
}

#[derive(Debug, Clone)]
pub struct EventAttributes<S> {
    timestamp: u64,
    source: S,

    source_type: TypeId,
}

impl<S> EventAttributes<S>
where
    S: Any,
{
    pub fn new(source: S) -> Self {
        // A clock set before the Unix epoch is rare, and it is not a reason to
        // fail the construction of an event: the timestamp of the event is then
        // the epoch itself.
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        Self {
            timestamp,
            source,
            source_type: TypeId::of::<S>(),
        }
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn source(&self) -> &dyn Any {
        &self.source
    }

    pub fn value(&self) -> &S {
        &self.source
    }

    pub fn source_type(&self) -> TypeId {
        self.source_type
    }
}

impl ApplicationEvent for Arc<dyn ApplicationEvent> {
    fn timestamp(&self) -> u64 {
        self.as_ref().timestamp()
    }

    /// Returns the source of the event as a `dyn Any` reference.
    fn source(&self) -> &dyn Any {
        self.as_ref().source()
    }

    fn event_type(&self) -> TypeId {
        self.as_ref().event_type()
    }

    fn source_type(&self) -> TypeId {
        self.as_ref().source_type()
    }
}
