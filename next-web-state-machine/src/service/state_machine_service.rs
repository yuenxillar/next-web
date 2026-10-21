use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::StateMachine;

/// Owns the machines of an application.
///
/// The service is the entry point an application uses instead of building a
/// machine itself: it returns the machine of an identifier, starts it, and stops
/// it again once the caller is done.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
#[async_trait]
pub trait StateMachineService<S, E>: Send + Sync
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Acquires the machine of an identifier and starts it.
    ///
    /// The first call builds the machine, later calls return the same instance.
    /// A machine which is restored from a store keeps the persisted state.
    ///
    /// # Arguments
    /// * `machine_id` - the identifier of the machine
    async fn acquire_state_machine(
        &self,
        machine_id: &str,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;

    /// Acquires the machine of an identifier and decides whether it is started.
    ///
    /// # Arguments
    /// * `machine_id` - the identifier of the machine
    /// * `start` - whether the machine starts before it is returned
    async fn acquire_state_machine_with_start(
        &self,
        machine_id: &str,
        start: bool,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError>;

    /// Releases the machine of an identifier.
    ///
    /// The machine is stopped and forgotten, so the next acquire builds a fresh
    /// one. Releasing a machine which is not acquired succeeds.
    ///
    /// # Arguments
    /// * `machine_id` - the identifier of the machine
    async fn release_state_machine(&self, machine_id: &str) -> Result<(), BoxError>;

    /// Releases the machine of an identifier and decides whether it is persisted.
    ///
    /// # Arguments
    /// * `machine_id` - the identifier of the machine
    /// * `persist` - whether the machine is written to the store before it stops
    async fn release_state_machine_with_persist(
        &self,
        machine_id: &str,
        persist: bool,
    ) -> Result<(), BoxError>;
}
