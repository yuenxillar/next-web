pub mod actions;

use next_web_core::{async_trait, error::BoxError};

use crate::state_context::StateContext;

#[async_trait]
pub trait StateMachineAction<S, E>
where
    Self: Send + Sync,
{
    async fn execute(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError>;
}
