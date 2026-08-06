use std::any::{Any, TypeId};

use dyn_clone::{DynClone, clone_trait_object};

/// Trait for application events that can be dispatched through the event system.
///
/// This trait provides common functionality for all events, including
/// timestamping and source identification. It is designed to be used with
/// trait objects (`dyn ApplicationEvent`), hence the Send + Sync and Any requirements.
///
/// # Requirements
///
/// - `Send + Sync`: Events must be safe to send and share across threads
/// - `Any`: Enables downcasting to concrete event types
/// - `DynClone`: Allows cloning of trait objects (requires the `dyn-clone` crate)
///
/// # Example
///
/// ```rust
/// #[derive(Clone)]
/// struct UserLoggedIn { user_id: u64 }
///
/// impl ApplicationEvent for UserLoggedIn {}
/// ```
pub trait ApplicationEvent
where
    Self: Send + Sync,
    Self: Any,
    Self: DynClone,
{
    /// Returns the timestamp when this event occurred.
    ///
    /// Default implementation returns 0, indicating no specific timestamp.
    /// Override this method to provide actual timing information.
    fn timestamp(&self) -> u64 {
        0
    }

    /// Returns the source of the event as a `dyn Any` reference.
    fn source(&self) -> &dyn Any;

    fn event_type(&self) -> TypeId;

    fn source_type(&self) -> TypeId;
}

clone_trait_object!(ApplicationEvent);

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
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
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

    pub fn source_type(&self) -> TypeId {
        self.source_type
    }
}

impl ApplicationEvent for Box<dyn ApplicationEvent> {
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
