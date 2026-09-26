use std::{
    error::Error,
    fmt::{self, Debug},
    sync::Arc,
    time::Duration,
};

use crate::{ApplicationEvent, ApplicationListener};

/// A listener that receives every event the multicaster publishes.
///
/// Listeners are type erased because the events they handle are not known when
/// they are registered: a listener is normally contributed by a provider, which
/// knows the type of the listener but not the events an application publishes.
/// Such a listener is responsible for ignoring the events it does not
/// understand. [`TypedApplicationListener`](crate::event::TypedApplicationListener)
/// is the adapter that does it for a listener of one concrete event type.
pub type ErasedApplicationListener = Arc<dyn ApplicationListener<Box<dyn ApplicationEvent>>>;

/// Multicasts application events to the registered listeners.
///
/// The methods take `&self` because a multicaster is shared as soon as it is
/// stored in an [`ApplicationContext`](crate::ApplicationContext): the context
/// keeps an `Arc<dyn ApplicationEventMulticaster>` and still has to be able to
/// add and remove listeners on the fly. Implementations therefore have to make
/// the set of listeners thread safe, which is what allows a listener to be
/// registered while another thread publishes an event.
pub trait ApplicationEventMulticaster
where
    Self: Send + Sync,
    Self: Debug,
{
    /// Adds the listener, replacing the listener registered under `id`.
    ///
    /// The listener receives every event the multicaster publishes.
    fn add_application_listener(&self, id: String, listener: ErasedApplicationListener);

    /// Removes the listener registered under `id`, when there is one.
    fn remove_application_listener(&self, id: String);

    /// Removes every listener.
    fn remove_all_listeners(&self);

    /// Publishes the event to the listeners that support it.
    ///
    /// The listeners are called in ascending order, and a listener that fails
    /// does not stop the remaining ones. How a failure is reported depends on
    /// the implementation: the
    /// [`DefaultApplicationEventMulticaster`](crate::event::DefaultApplicationEventMulticaster)
    /// hands it to its error handler and, unless its failure policy is
    /// [`Propagate`](crate::event::ListenerFailurePolicy::Propagate), still
    /// reports success.
    fn multicast_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), MulticastError>;
}

/// A listener that failed while it handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListenerFailure {
    /// Identifier the listener was registered under.
    pub listener_id: String,
    /// Description of the failure, normally the message of the panic the
    /// listener raised.
    pub message: String,
}

impl ListenerFailure {
    /// Creates the failure of the listener registered under `listener_id`.
    ///
    /// # Arguments
    ///
    /// * `listener_id` - The identifier the listener was registered under.
    /// * `message` - The description of the failure.
    pub fn new(listener_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            listener_id: listener_id.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ListenerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.listener_id, self.message)
    }
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

    /// One or more listeners failed while they handled the event
    ListenerFailures(Vec<ListenerFailure>),

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
            MulticastError::ListenerFailures(failures) => {
                write!(f, "{} listener(s) failed: ", failures.len())?;

                let mut failures = failures.iter();
                if let Some(first) = failures.next() {
                    write!(f, "{first}")?;
                }
                for failure in failures {
                    write!(f, "; {failure}")?;
                }

                Ok(())
            }
            MulticastError::Other(msg) => {
                write!(f, "Multicast error: {}", msg)
            }
        }
    }
}

impl Error for MulticastError {}
