use std::time::Duration;

use next_web_core::async_trait;

use crate::state_machine::{BoxedStateAction, StateMachine};

/// `ActionListener` for various action events.
///
/// This trait defines callbacks for monitoring action execution within
/// a state machine, particularly focusing on timing and performance
/// metrics for action functions.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait ActionListener<S, E>
where
    Self: Send + Sync,
{
    /// Notified during execution of a particular action.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine
    /// * `action` - the action function being executed
    /// * `duration` - the action execution duration
    async fn on_execute(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &BoxedStateAction<S, E>,
        duration: Duration,
    );
}
