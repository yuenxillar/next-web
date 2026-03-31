pub mod composite_trigger_listener;
pub mod timer_trigger;
pub mod trigger_context;
pub mod trigger_listener;

use std::{any::Any, sync::Arc};

use next_web_core::{DynClone, async_trait, clone_trait_object, traits::id::Id};

use crate::trigger::{trigger_context::TriggerContext, trigger_listener::TriggerListener};

/// `Trigger` is the cause of the `Transition`. Cause is usually an
/// event but can be some other signal or a change in some condition.
///
/// This trait defines the interface for triggers that initiate state
/// transitions in a state machine. Triggers can be event-based,
/// timer-based, or condition-based.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait Trigger<S, E>
where
    Self: Id,
    Self: Any,
    Self: Send + Sync,
    Self: DynClone,
{
    /// Evaluate trigger.
    ///
    /// # Arguments
    /// * `context` - the trigger context
    ///
    /// # Returns
    /// A future that resolves to `true` if trigger is fired, `false` otherwise
    async fn evaluate(&self, context: &dyn TriggerContext<S, E>) -> bool;

    /// Adds the trigger listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    async fn add_trigger_listener(&self, listener: Arc<dyn TriggerListener>);

    /// Gets the event associated with this trigger. It is possible that there
    /// are no event association.
    ///
    /// # Returns
    /// The event associated with this trigger, if any
    fn event(&self) -> Option<&E>;

    /// Arm a trigger. After trigger has been armed a `TriggerListener`
    /// may receive events.
    async fn arm(&mut self);

    /// Disarm a trigger. After trigger has been disarmed a `TriggerListener`
    /// will not receive events.
    async fn disarm(&mut self);
}

clone_trait_object!(<S, E> Trigger<S, E>
where
    Self: Send + Sync,
);
