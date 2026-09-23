use std::time::Duration;

use async_trait::async_trait;

use super::application_listener::ApplicationListener;
use crate::traits::event::application_event::ApplicationEvent;

/// Application event multicaster
#[async_trait]
pub trait ApplicationEventMulticaster
where
    Self: Send + Sync,
{
    /// Add application event listener
    async fn add_application_listener<L, E>(&mut self, id: String, listener: L)
    where
        L: ApplicationListener<E>,
        E: ApplicationEvent;

    /// Remove application event listener
    async fn remove_application_listener<E>(&mut self, id: String)
    where
        E: ApplicationEvent;

    /// Remove all application event listeners
    async fn remove_all_listeners(&mut self);

    /// Multicast application event
    async fn multicast_event<E>(&self, event: E) -> Result<(), MulticastError>
    where
        E: ApplicationEvent;
}

/// Errors that can occur during event multicasting
#[derive(Debug, thiserror::Error)]
pub enum MulticastError {
    /// No listeners registered for this event type
    #[error("No listeners registered for event type: {0}")]
    NoListeners(String),

    /// Failed to send event to processor channel
    #[error("Failed to send event: {0}")]
    SendError(String),

    /// Send operation timed out
    #[error("Send operation timed out after {0:?}")]
    Timeout(Duration),

    /// Event processor channel is closed
    #[error("Event processor channel is closed")]
    ChannelClosed,

    /// Failed to downcast event to expected type
    #[error("Failed to downcast event")]
    DowncastError,

    /// Listener execution failed
    #[error("Listener execution failed: {0}")]
    ListenerError(String),

    /// Other errors
    #[error("Multicast error: {0}")]
    Other(String),
}
