use crate::state_machine::state_context::StateContext;

/// Guards are typically considered as guard conditions which affect the behaviour of a state machine by
/// enabling actions or transitions only when they evaluate to TRUE and disabling them when they evaluate to FALSE.
pub trait StateMachineGuard<S, E>
where
    Self: Send + Sync,
{
    /// Evaluate a guard condition.
    fn evaluate(&self, ctx: &dyn StateContext<S, E>);
}
