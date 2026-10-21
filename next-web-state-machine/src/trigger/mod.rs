//! Triggers are the cause of a transition.

/// Listener composite of the triggers.
pub mod composite_trigger_listener;
/// The default trigger context.
pub mod default_trigger_context;
/// The trigger which fires from an event.
pub mod event_trigger;
/// The trigger which fires from a timer.
pub mod timer_trigger;
/// The trigger context contract.
pub mod trigger_context;
/// The trigger listener contract.
pub mod trigger_listener;

use std::any::Any;
use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::{DynClone, Id, clone_trait_object};

use crate::trigger::{trigger_context::TriggerContext, trigger_listener::TriggerListener};

/// `Trigger` is the cause of the `Transition`. Cause is usually an
/// event but can be some other signal or a change in some condition.
///
/// Trigger listeners are notified from possibly unrelated tasks (a timer for
/// example), so a trigger owns its interior mutability instead of requiring
/// `&mut self`: arming and disarming happen while the state machine only holds
/// a shared reference.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
pub trait Trigger<S, E>: Id + Any + Send + Sync + DynClone {
    /// Evaluates the trigger.
    ///
    /// # Arguments
    /// * `context` - the trigger context
    ///
    /// # Returns
    /// A future that resolves to `true` if the trigger is fired
    fn evaluate<'a>(&'a self, context: &'a dyn TriggerContext<S, E>) -> BoxFuture<'a, bool>;

    /// Adds the trigger listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_trigger_listener(&self, listener: Arc<dyn TriggerListener>);

    /// Gets the event associated with this trigger. It is possible that there
    /// is no event association.
    fn event(&self) -> Option<&E>;

    /// Arms the trigger. After a trigger has been armed a
    /// [`TriggerListener`] may receive events.
    fn arm(&self);

    /// Disarms the trigger. After a trigger has been disarmed a
    /// [`TriggerListener`] will not receive events.
    fn disarm(&self);
}

clone_trait_object!(<S, E> Trigger<S, E>
where
    Self: Send + Sync,
);
