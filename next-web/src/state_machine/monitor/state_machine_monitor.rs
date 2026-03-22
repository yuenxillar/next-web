use std::time::Duration;

use futures::future::BoxFuture;

use crate::state_machine::{
    state_context::StateContext, transition::StateMachineTransition, StateMachine,
};

/// StateMachineMonitor for various state machine monitoring events.
pub trait StateMachineMonitor<S, E>
where
    Self: Send + Sync,
{
    ///  Notified duration of a particular transition.
    fn transition(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        transition: &dyn StateMachineTransition<S, E>,
        duration: Duration,
    );

    /// Notified duration of a particular action.
    fn action(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>,
        duration: Duration,
    );
}
