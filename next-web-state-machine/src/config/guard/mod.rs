use next_web_core::async_trait;

use crate::state_context::StateContext;

/// Guards are typically considered as guard conditions which affect the
/// behaviour of a state machine by enabling actions or transitions only when
/// they evaluate to TRUE and disabling them when they evaluate to FALSE.
#[async_trait]
pub trait StateMachineGuard<S, E>
where
    Self: Send + Sync,
{
    /// Evaluates the guard condition.
    ///
    /// # Arguments
    /// * `ctx` - the state context
    ///
    /// # Returns
    /// `true` when the guard allows the action or transition
    async fn evaluate(&self, ctx: &dyn StateContext<S, E>) -> bool;
}
