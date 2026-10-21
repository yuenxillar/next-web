use std::time::Duration;

use crate::BoxedStateAction;
use crate::{StateMachine, transition::StateMachineTransition};

/// StateMachineMonitor for various state machine monitoring events.
pub trait StateMachineMonitor<S, E>
where
    Self: Send + Sync,
{
    /// Notified duration of a particular transition.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `transition` - the transition which ran
    /// * `duration` - how long the transition took
    fn transition(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        transition: &dyn StateMachineTransition<S, E>,
        duration: Duration,
    );

    /// Notified duration of a particular action.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `action` - the action which ran
    /// * `duration` - how long the action took
    fn action(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &BoxedStateAction<S, E>,
        duration: Duration,
    );
}
