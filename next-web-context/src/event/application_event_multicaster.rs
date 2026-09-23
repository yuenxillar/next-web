use std::{
    error::Error,
    fmt::{self, Debug},
    sync::Arc,
    time::Duration,
};

use crate::{ApplicationEvent, ApplicationListener};

/// Application event multicaster
pub trait ApplicationEventMulticaster
where
    Self: Send + Sync,
    Self: Debug,
{
    /// Add application event listener
    fn add_application_listener(
        &mut self,
        id: String,
        listener: Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>,
    );

    /// Remove application event listener
    fn remove_application_listener(&mut self, id: String);

    /// Remove all application event listeners
    fn remove_all_listeners(&mut self);

    /// Multicast application event
    fn multicast_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), MulticastError>;
}

/// Errors that can occur during event multicasting
#[derive(Debug)]
pub enum MulticastError {
    /// No listeners registered for this event type
    NoListeners(String),

    /// Failed to send event to processor channel
    SendError(String),

    /// Send operation timed out
    Timeout(Duration),

    /// Event processor channel is closed
    ChannelClosed,

    /// Failed to downcast event to expected type
    DowncastError,

    /// Listener execution failed
    ListenerError(String),

    /// Other errors
    Other(String),
}

impl fmt::Display for MulticastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MulticastError::NoListeners(event_type) => {
                write!(f, "No listeners registered for event type: {}", event_type)
            }
            MulticastError::SendError(msg) => {
                write!(f, "Failed to send event: {}", msg)
            }
            MulticastError::Timeout(duration) => {
                write!(f, "Send operation timed out after {:?}", duration)
            }
            MulticastError::ChannelClosed => {
                write!(f, "Event processor channel is closed")
            }
            MulticastError::DowncastError => {
                write!(f, "Failed to downcast event")
            }
            MulticastError::ListenerError(msg) => {
                write!(f, "Listener execution failed: {}", msg)
            }
            MulticastError::Other(msg) => {
                write!(f, "Multicast error: {}", msg)
            }
        }
    }
}

impl Error for MulticastError {}
