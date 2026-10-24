//! Actions configured through the model.
/// The built-in actions, such as the ones which change the extended state.
pub mod actions;

use next_web_core::{async_trait, error::BoxError};

use crate::state_context::StateContext;

/// An action executed by a state or a transition.
#[async_trait]
pub trait StateMachineAction<S, E>
where
    Self: Send + Sync,
{
    /// Executes the action.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn execute(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError>;
}
