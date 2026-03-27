use next_web_core::{async_trait, error::BoxError};

use crate::StateMachine;

#[async_trait]
pub trait StateMachinePersister<S, E, T> {
    /// Persist the state machine and its associated data.
    async fn persist<T1>(&mut self, var1: T1, var2: T) -> Result<(), BoxError>
    where
        T1: StateMachine<S, E>;

    /// Restore the state machine and its associated data.
    async fn restore<T1>(&mut self, var1: T) -> Result<T1, BoxError>
    where
        T1: StateMachine<S, E>;
}
