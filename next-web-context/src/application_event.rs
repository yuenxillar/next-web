use std::any::Any;

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
}

clone_trait_object!(ApplicationEvent);

#[derive(Debug, Clone)]
pub struct EventAttributes<S> {
    timestamp: u64,
    source: S,
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
        Self { timestamp, source }
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn source(&self) -> &dyn Any {
        &self.source
    }
}
