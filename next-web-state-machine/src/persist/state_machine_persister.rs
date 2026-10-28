use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::StateMachine;

/// Persists and restores a state machine together with the data which describes
/// what it was doing.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
/// * `T` - the type of the context which is stored alongside the machine
#[async_trait]
pub trait StateMachinePersister<S, E, T>: Send + Sync
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// Persists the state machine and its context.
    ///
    /// # Arguments
    /// * `state_machine` - the state machine to persist
    /// * `context` - the context which describes the machine
    async fn persist(
        &self,
        state_machine: Arc<dyn StateMachine<S, E>>,
        context: T,
    ) -> Result<(), BoxError>;

    /// Restores a state machine from its context.
    ///
    /// # Arguments
    /// * `context` - the context which describes the machine
    async fn restore(&self, context: T) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;
}
