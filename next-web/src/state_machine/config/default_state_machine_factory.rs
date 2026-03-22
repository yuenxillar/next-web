use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

// use crate::state_machine::config::base_state_machine_factory::BaseStateMachineFactory;

pub struct DefaultStateMachineFactory<S, E> {
    // base: BaseStateMachineFactory<S, E>,
    //
    var: PhantomData<(S, E)>,
}

impl<S, E> DefaultStateMachineFactory<S, E> {}

// impl<S, E> Deref for DefaultStateMachineFactory<S, E> {
//     type Target = BaseStateMachineFactory<S, E>;

//     fn deref(&self) -> &Self::Target {
//         &self.base
//     }
// }

// impl<S, E> DerefMut for DefaultStateMachineFactory<S, E> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.base
//     }
// }
