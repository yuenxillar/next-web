use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::{
    StateMachine,
    config::{
        state_machine_configure::StateMachineConfigure,
        state_machine_state_configure::StateMachineStateConfigure,
        state_machine_transition_configure::StateMachineTransitionConfigure,
    },
};

pub struct StateMachineGenerator;

impl StateMachineGenerator {
    pub fn generate<S, E>(
        id: impl ToString,
        transition_configure: StateMachineTransitionConfigure<S, E>,
    ) -> Arc<dyn StateMachine<S, E>>
    where
        S: Default + Clone + Debug + Hash + Eq + PartialEq,
        S: Send + Sync + 'static,
        E: Clone + Debug + Hash + Eq + PartialEq,
        E: Send + Sync + 'static,
    {
        let configure = StateMachineConfigure::<S, E>::default();
        // let state_configure = StateMachineStateConfigure::default();

        // let state_machie =
        //     StateMachine::from_configure(configure, state_configure, transition_configure)
        //         .set_id(id.to_string());

        unimplemented!()
    }
}
