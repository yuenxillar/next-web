//! An interceptor which persists the machine whenever its state changes.

use std::sync::Arc;

use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tracing::warn;

use crate::StateMachine;
use crate::state::StateMachineState;
use crate::state_context::StateContext;
use crate::state_machine_persist::StateMachinePersist;
use crate::support::default_state_machine_context::DefaultStateMachineContext;
use crate::support::state_machine_interceptor::StateMachineInterceptor;
use crate::transition::StateMachineTransition;

/// Persists the state of the machine after every state change.
///
/// The Java original writes on `preStateChange` for transitions and on the
/// initial transition of a resumed machine. The Rust port writes on
/// `post_state_change`, so the store always holds a state the machine really
/// entered.
///
/// The extended state variables are not captured by this interceptor; use
/// [`crate::persist::default_state_machine_persister::DefaultStateMachinePersister`]
/// when they matter.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
/// * `T` - the type of the store key
pub struct PersistingStateMachineInterceptor<S, E, T> {
    persist: Arc<dyn StateMachinePersist<S, E, T>>,
    context_obj: T,
}

impl<S, E, T> PersistingStateMachineInterceptor<S, E, T> {
    /// Creates a new interceptor.
    ///
    /// # Arguments
    /// * `persist` - the store which keeps the machine context
    /// * `context_obj` - the key the context is stored under
    pub fn new(persist: Arc<dyn StateMachinePersist<S, E, T>>, context_obj: T) -> Self {
        Self {
            persist,
            context_obj,
        }
    }
}

impl<S, E, T> StateMachineInterceptor<S, E> for PersistingStateMachineInterceptor<S, E, T>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    fn pre_event(
        &self,
        message: Arc<dyn Message<E>>,
        _state_machine: &dyn StateMachine<S, E>,
    ) -> Result<Arc<dyn Message<E>>, BoxError> {
        Ok(message)
    }

    fn pre_state_change(
        &self,
        _state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        _state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) -> Result<(), BoxError> {
        Ok(())
    }

    fn post_state_change(
        &self,
        state: &dyn StateMachineState<S, E>,
        _message: Option<&dyn Message<E>>,
        _transition: Option<&dyn StateMachineTransition<S, E>>,
        state_machine: &dyn StateMachine<S, E>,
        _root_state_machine: &dyn StateMachine<S, E>,
    ) {
        let context = DefaultStateMachineContext::new(
            state_machine.id().to_owned(),
            state.id().clone(),
        );

        if let Err(error) = self.persist.write(&context, self.context_obj.clone()) {
            warn!(%error, "persisting the state machine failed");
        }
    }

    fn pre_transition(&self, _state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn post_transition(&self, _state_context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn state_machine_error(
        &self,
        _state_machine: &dyn StateMachine<S, E>,
        error: BoxError,
    ) -> Option<BoxError> {
        Some(error)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use next_web_core::error::BoxError;
    use next_web_core::messaging::generic_message::GenericMessage;

    use crate::StateMachine as _;
    use crate::config::base_state_machine_factory::BaseStateMachineFactory;
    use crate::config::model::configuration_data::ConfigurationData;
    use crate::config::model::default_state_machine_model::DefaultStateMachineModel;
    use crate::config::model::state_data::StateData;
    use crate::config::model::transition_data::TransitionData;
    use crate::config::model::transitions_data::TransitionsData;
    use crate::config::state_machine_factory::StateMachineFactory;
    use crate::persist::persisting_state_machine_interceptor::PersistingStateMachineInterceptor;
    use crate::region::Region;
    use crate::state_machine_context::StateMachineContext;
    use crate::state_machine_persist::StateMachinePersist;

    #[derive(Default)]
    struct MemoryPersist {
        slot: Mutex<Option<(String, i32)>>,
    }

    impl StateMachinePersist<i32, i32, ()> for MemoryPersist {
        fn write(
            &self,
            context: &dyn StateMachineContext<i32, i32>,
            _context_obj: (),
        ) -> Result<(), BoxError> {
            *self.slot.lock().expect("lock") =
                Some((context.id().to_owned(), *context.state()));
            Ok(())
        }

        fn read(
            &self,
            _context_obj: (),
        ) -> Result<Arc<dyn StateMachineContext<i32, i32>>, BoxError> {
            Err(Box::new(std::io::Error::other("read is not used")))
        }
    }

    #[tokio::test]
    async fn every_state_change_is_persisted() {
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
            BaseStateMachineFactory::<i32, i32>::new(Arc::new(model), None);
        let machine: Arc<dyn crate::StateMachine<i32, i32>> =
            factory.get_state_machine().expect("machine builds");

        let persist = Arc::new(MemoryPersist::default());
        machine
            .add_state_machine_interceptor(Arc::new(
                PersistingStateMachineInterceptor::new(persist.clone(), ()),
            ))
            .expect("interceptor registers");

        machine.start().await.expect("start succeeds");
        machine
            .send_event(Arc::new(GenericMessage::with_payload(100)))
            .await
            .expect("event is sent");

        let stored = persist.slot.lock().expect("lock").clone();
        assert_eq!(
            stored.map(|(_, state)| state),
            Some(2),
            "the interceptor wrote the state the machine entered"
        );
    }
}
