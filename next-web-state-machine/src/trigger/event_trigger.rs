//! The event trigger matches an event by equality.

use std::marker::PhantomData;
use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::Id;

use crate::trigger::trigger_context::TriggerContext;
use crate::trigger::trigger_listener::TriggerListener;
use crate::trigger::Trigger;

/// A trigger which fires when the event of the trigger context equals the event
/// of the trigger.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct EventTrigger<S, E> {
    event: E,
    _marker: PhantomData<S>,
}

impl<S, E> EventTrigger<S, E> {
    /// Creates a new event trigger.
    ///
    /// # Arguments
    /// * `event` - the event which fires this trigger
    pub fn new(event: E) -> Self {
        Self {
            event,
            _marker: PhantomData,
        }
    }
}

impl<S, E> Clone for EventTrigger<S, E>
where
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            event: self.event.clone(),
            _marker: PhantomData,
        }
    }
}

impl<S, E> Id for EventTrigger<S, E> {
    fn id(&self) -> &str {
        "eventTrigger"
    }
}

impl<S, E> Trigger<S, E> for EventTrigger<S, E>
where
    S: Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    fn evaluate<'a>(&'a self, context: &'a dyn TriggerContext<S, E>) -> BoxFuture<'a, bool> {
        Box::pin(async move { context.get_event() == &self.event })
    }

    fn add_trigger_listener(&self, _listener: Arc<dyn TriggerListener>) {}

    fn event(&self) -> Option<&E> {
        Some(&self.event)
    }

    fn arm(&self) {}

    fn disarm(&self) {}
}
