//! The default persister of a state machine.

use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::StateMachine;
use crate::config::state_machine_factory::StateMachineFactory;
use crate::persist::state_machine_persister::StateMachinePersister;
use crate::state_machine_persist::StateMachinePersist;

/// Persists and restores a state machine.
///
/// The machine is captured as a [`crate::state_machine_context::StateMachineContext`]
/// and written through a [`StateMachinePersist`]. Restoring builds a fresh
/// machine through the factory and resets it to the persisted state without
/// running entry or exit actions, which is what the Java implementation does.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
/// * `T` - the type of the store key
pub struct DefaultStateMachinePersister<S, E, T> {
    factory: Arc<dyn StateMachineFactory<S, E>>,
    persist: Arc<dyn StateMachinePersist<S, E, T>>,
}

impl<S, E, T> DefaultStateMachinePersister<S, E, T> {
    /// Creates a new persister.
    ///
    /// # Arguments
    /// * `factory` - the factory which builds the machine that is restored
    /// * `persist` - the store which keeps the machine context
    pub fn new(
        factory: Arc<dyn StateMachineFactory<S, E>>,
        persist: Arc<dyn StateMachinePersist<S, E, T>>,
    ) -> Self {
        Self { factory, persist }
    }
}

#[async_trait]
impl<S, E, T> StateMachinePersister<S, E, T> for DefaultStateMachinePersister<S, E, T>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    async fn persist(
        &self,
        state_machine: Arc<dyn StateMachine<S, E>>,
        context: T,
    ) -> Result<(), BoxError> {
        let machine_context = state_machine.snapshot_context()?;
        self.persist.write(machine_context.as_ref(), context)
    }

    async fn restore(&self, context: T) -> Result<Arc<dyn StateMachine<S, E>>, BoxError> {
        let machine_context = self.persist.read(context)?;

        let state_machine = self
            .factory
            .get_state_machine_with_id(machine_context.id().to_owned())?;

        // The context restores the state and the extended state variables it
        // captured when the machine was persisted.
        state_machine.restore_context(machine_context.as_ref())?;

        Ok(state_machine)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use next_web_core::error::BoxError;
    use next_web_core::messaging::generic_message::GenericMessage;

    use crate::config::model::configuration_data::ConfigurationData;
    use crate::config::model::default_state_machine_model::DefaultStateMachineModel;
    use crate::config::model::state_data::StateData;
    use crate::config::model::transition_data::TransitionData;
    use crate::config::model::transitions_data::TransitionsData;
    use crate::config::state_machine_factory::StateMachineFactory;
    use crate::persist::default_state_machine_persister::DefaultStateMachinePersister;
    use crate::persist::state_machine_persister::StateMachinePersister;
    use crate::state_machine_context::StateMachineContext;
    use crate::state_machine_persist::StateMachinePersist;
    use crate::support::default_state_machine_context::DefaultStateMachineContext;
    

    /// An in-memory store used to prove the persist and restore round trip.
    #[derive(Default)]
    struct MemoryPersist {
        slot: Mutex<Option<(String, i32, HashMap<String, next_web_core::any::AnyValue>)>>,
    }

    impl StateMachinePersist<i32, i32, ()> for MemoryPersist {
        fn write(
            &self,
            context: &dyn StateMachineContext<i32, i32>,
            _context_obj: (),
        ) -> Result<(), BoxError> {
            *self.slot.lock().expect("lock") = Some((
                context.id().to_owned(),
                *context.state(),
                context.extended_state().get_variables(),
            ));
            Ok(())
        }

        fn read(&self, _context_obj: ()) -> Result<Arc<dyn StateMachineContext<i32, i32>>, BoxError> {
            let stored = self.slot.lock().expect("lock").clone();

            match stored {
                Some((id, state, variables)) => {
                    let extended_state: Arc<dyn crate::extended_state::ExtendedState> =
                        Arc::new(crate::support::default_extended_state::DefaultExtendedState::with_variables(variables));
                    Ok(Arc::new(
                        DefaultStateMachineContext::new(id, state)
                            .with_extended_state(extended_state),
                    ))
                }
                None => Err(crate::state_machine_exception::state_machine_error(
                    crate::state_machine_exception::StateMachineErrorKind::MachineNotFound,
                    "nothing was persisted",
                )),
            }
        }
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn a_machine_state_is_persisted_and_restored() {
        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory =
            Arc::new(crate::config::base_state_machine_factory::BaseStateMachineFactory::<
                i32,
                i32,
            >::new(Arc::new(model), None));

        let persister = DefaultStateMachinePersister::new(
            factory.clone() as Arc<dyn StateMachineFactory<i32, i32>>,
            Arc::new(MemoryPersist::default()),
        );

        let machine = factory.get_state_machine().expect("machine builds");
        machine.start().await.expect("start succeeds");
        machine
            .send_event(Arc::new(GenericMessage::with_payload(100)))
            .await
            .expect("event is sent");
        assert_eq!(machine.state().map(|state| *state.id()), Some(2));

        persister
            .persist(machine.clone(), ())
            .await
            .expect("the machine is persisted");

        let restored = persister.restore(()).await.expect("the machine is restored");
        assert_eq!(
            restored.state().map(|state| *state.id()),
            Some(2),
            "the restored machine continues in the persisted state"
        );
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn the_extended_state_variables_are_restored() {
        use next_web_core::any::AnyValue;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::new(1, 2, 100)])),
        );

        let factory =
            Arc::new(crate::config::base_state_machine_factory::BaseStateMachineFactory::<
                i32,
                i32,
            >::new(Arc::new(model), None));

        let persister = DefaultStateMachinePersister::new(
            factory.clone() as Arc<dyn StateMachineFactory<i32, i32>>,
            Arc::new(MemoryPersist::default()),
        );

        let machine = factory.get_state_machine().expect("machine builds");
        machine.start().await.expect("start succeeds");
        machine
            .send_event(Arc::new(GenericMessage::with_payload(100)))
            .await
            .expect("event is sent");

        machine
            .extended_state()
            .set("tenant".to_owned(), AnyValue::from("acme"));

        persister
            .persist(machine.clone(), ())
            .await
            .expect("the machine is persisted");

        let restored = persister.restore(()).await.expect("the machine is restored");

        assert_eq!(
            restored.state().map(|state| *state.id()),
            Some(2),
            "the state is restored"
        );
        assert_eq!(
            restored
                .extended_state()
                .get(&"tenant".to_owned())
                .and_then(|value| value.as_str().map(str::to_owned)),
            Some("acme".to_owned()),
            "the extended state variable is restored"
        );
    }

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn a_restored_machine_fires_its_timer() {
        use crate::transition::transition_kind::TransitionKind;

        let mut configuration = ConfigurationData::default();
        configuration.verifier_enabled = false;

        let model = DefaultStateMachineModel::new(
            configuration,
            Some(
                vec![
                    StateData::<i32, i32>::with_initial(1, true),
                    StateData::<i32, i32>::new(2),
                ]
                .into(),
            ),
            Some(TransitionsData::new(vec![TransitionData::with_all(
                1,
                2,
                None,
                None,
                Some(200),
                Some(1),
                Vec::new(),
                None,
                TransitionKind::External,
                None,
                "timer",
            )])),
        );

        let factory =
            Arc::new(crate::config::base_state_machine_factory::BaseStateMachineFactory::<
                i32,
                i32,
            >::new(Arc::new(model), None));

        let persister = DefaultStateMachinePersister::new(
            factory.clone() as Arc<dyn StateMachineFactory<i32, i32>>,
            Arc::new(MemoryPersist::default()),
        );

        let machine = factory.get_state_machine().expect("machine builds");
        machine.start().await.expect("start succeeds");
        assert_eq!(machine.state().map(|state| *state.id()), Some(1));

        persister
            .persist(machine.clone(), ())
            .await
            .expect("the machine is persisted");

        let restored = persister.restore(()).await.expect("the machine is restored");
        assert_eq!(
            restored.state().map(|state| *state.id()),
            Some(1),
            "the restored machine is on the persisted state"
        );

        restored.start().await.expect("the restored machine starts");
        tokio::time::sleep(std::time::Duration::from_millis(700)).await;

        assert_eq!(
            restored.state().map(|state| *state.id()),
            Some(2),
            "the timer of the restored machine fires"
        );
    }
}
