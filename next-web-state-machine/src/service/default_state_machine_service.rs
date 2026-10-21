use std::collections::HashMap;
use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};
use tokio::sync::Mutex;
use tracing::debug;

use crate::StateMachine;
use crate::config::state_machine_factory::StateMachineFactory;
use crate::persist::state_machine_persister::StateMachinePersister;
use crate::service::state_machine_service::StateMachineService;
use crate::state_machine_exception::{StateMachineErrorKind, StateMachineException};

/// The default service, which keeps the acquired machines in memory.
///
/// The machines are cached by identifier and removed when they are released. A
/// configured persister restores a machine when it is acquired and writes it
/// before it is released.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct DefaultStateMachineService<S, E> {
    state_machine_factory: Arc<dyn StateMachineFactory<S, E>>,
    persister: Option<Arc<dyn StateMachinePersister<S, E, String>>>,
    machines: Mutex<HashMap<String, Arc<dyn StateMachine<S, E>>>>,
    start_enabled: bool,
}

impl<S, E> DefaultStateMachineService<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new service.
    ///
    /// # Arguments
    /// * `state_machine_factory` - the factory which builds the machines
    pub fn new(state_machine_factory: Arc<dyn StateMachineFactory<S, E>>) -> Self {
        Self {
            state_machine_factory,
            persister: None,
            machines: Mutex::new(HashMap::new()),
            start_enabled: true,
        }
    }

    /// Sets the persister of the service.
    ///
    /// # Arguments
    /// * `persister` - the persister which stores and restores a machine by id
    pub fn with_persister(
        mut self,
        persister: Arc<dyn StateMachinePersister<S, E, String>>,
    ) -> Self {
        self.persister = Some(persister);
        self
    }

    /// Sets whether an acquired machine is started.
    ///
    /// # Arguments
    /// * `start_enabled` - whether the machines start when they are acquired
    pub fn with_start_enabled(mut self, start_enabled: bool) -> Self {
        self.start_enabled = start_enabled;
        self
    }

    /// Builds the machine of an identifier, restoring it when a state exists.
    ///
    /// # Arguments
    /// * `machine_id` - the identifier of the machine
    async fn build_state_machine(
        &self,
        machine_id: &str,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let Some(persister) = self.persister.as_ref() else {
            return self
                .state_machine_factory
                .get_state_machine_with_id(machine_id.to_owned());
        };

        match persister.restore(machine_id.to_owned()).await {
            Ok(machine) => Ok(machine),
            Err(error) if is_machine_not_found(error.as_ref()) => {
                debug!(
                    machine_id = machine_id,
                    "no persisted state exists, the machine is built from its configuration"
                );

                self.state_machine_factory
                    .get_state_machine_with_id(machine_id.to_owned())
            }
            Err(error) => Err(error),
        }
    }

    /// Starts the machine when it is not running yet.
    ///
    /// # Arguments
    /// * `machine` - the machine to start
    async fn start_state_machine(
        &self,
        machine: &Arc<dyn StateMachine<S, E>>,
    ) -> Result<(), BoxError> {
        if machine.is_running() {
            return Ok(());
        }

        machine.start().await
    }
}

#[async_trait]
impl<S, E> StateMachineService<S, E> for DefaultStateMachineService<S, E>
where
    S: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    E: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
{
    async fn acquire_state_machine(
        &self,
        machine_id: &str,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        self.acquire_state_machine_with_start(machine_id, self.start_enabled)
            .await
    }

    async fn acquire_state_machine_with_start(
        &self,
        machine_id: &str,
        start: bool,
    ) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        // The lock is held across the build, so two callers which acquire the
        // same identifier at the same time share one machine.
        let mut machines = self.machines.lock().await;

        if let Some(machine) = machines.get(machine_id).cloned() {
            if start {
                self.start_state_machine(&machine).await?;
            }

            return Ok(machine);
        }

        let machine = self.build_state_machine(machine_id).await?;

        if start {
            self.start_state_machine(&machine).await?;
        }

        machines.insert(machine_id.to_owned(), machine.clone());

        Ok(machine)
    }

    async fn release_state_machine(&self, machine_id: &str) -> Result<(), BoxError> {
        self.release_state_machine_with_persist(machine_id, self.persister.is_some())
            .await
    }

    async fn release_state_machine_with_persist(
        &self,
        machine_id: &str,
        persist: bool,
    ) -> Result<(), BoxError> {
        let machine = self.machines.lock().await.remove(machine_id);

        let Some(machine) = machine else {
            return Ok(());
        };

        // The machine is written before it stops, because stopping clears the
        // current state which the persisted context captures.
        if persist && let Some(persister) = self.persister.as_ref() {
            persister
                .persist(machine.clone(), machine_id.to_owned())
                .await?;
        }

        machine.stop().await
    }
}

/// Checks whether an error reports that no persisted state exists.
///
/// # Arguments
/// * `error` - the error raised while restoring
fn is_machine_not_found(error: &(dyn std::error::Error + 'static)) -> bool {
    error
        .downcast_ref::<StateMachineException>()
        .map(|exception| exception.kind() == StateMachineErrorKind::MachineNotFound)
        .unwrap_or(false)
}
