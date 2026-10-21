//! A trigger context carrying the event which caused the evaluation.

use crate::trigger::trigger_context::TriggerContext;

/// Default implementation of [`TriggerContext`].
///
/// The event is borrowed from the message which is being processed, so
/// evaluating a trigger does not clone the event.
pub struct DefaultTriggerContext<'a, S, E> {
    event: &'a E,
    _marker: std::marker::PhantomData<S>,
}

impl<'a, S, E> DefaultTriggerContext<'a, S, E> {
    /// Creates a new trigger context.
    ///
    /// # Arguments
    /// * `event` - the event which caused the evaluation
    pub fn new(event: &'a E) -> Self {
        Self {
            event,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, E> TriggerContext<S, E> for DefaultTriggerContext<'_, S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn get_event(&self) -> &E {
        self.event
    }
}
