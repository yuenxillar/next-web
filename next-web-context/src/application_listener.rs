use std::any::Any;

use crate::{ApplicationEvent, BoxFuture};

/// Handles the events of the application.
///
/// A listener is registered for the events of the whole application
/// (`E = Arc<dyn ApplicationEvent>`) or for the events of one type (`E = OrderPlaced`,
/// with the adapter of [`TypedApplicationListener`](crate::event::TypedApplicationListener)),
/// and the event is handed to it **borrowed**: the event system publishes one shared
/// value and every listener reads it, so a listener does not clone the event and an
/// event does not have to be cloneable.
///
/// A listener of one event type names the type it handles, and the downcast happens
/// where the adapter hands it the event — the listener reads the event it asked for:
///
/// ```rust,ignore
/// use next_web_context::{ApplicationEvent, ApplicationListener, BoxFuture};
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
/// struct Audit;
///
/// impl ApplicationListener<OrderPlaced> for Audit {
///     fn on_application_event<'a>(&'a self, order: &'a OrderPlaced) -> BoxFuture<'a, ()> {
///         Box::pin(async move {
///             println!("the order {} was placed", order.id);
///         })
///     }
/// }
/// ```
pub trait ApplicationListener<E>
where
    E: ApplicationEvent,
    Self: Any,
    Self: Send + Sync,
{
    /// Handles the event, which the event system keeps alive while the future runs.
    ///
    /// # Arguments
    ///
    /// * `event` - The event that is published, borrowed from the shared value of it.
    fn on_application_event<'a>(&'a self, event: &'a E) -> BoxFuture<'a, ()>;
}
