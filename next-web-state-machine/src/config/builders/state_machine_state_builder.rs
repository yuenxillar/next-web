use std::{
    cell::RefCell,
    hash::Hash,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use next_web_core::{error::BoxError, traits::required::Required};

use crate::config::{
    builders::state_machine_state_configurer::StateMachineStateConfigurer,
    common::{
        base_builder::{BaseBuilder, BaseBuilderExt},
        base_configured_builder::{
            BaseConfiguredBuilder, BaseConfiguredBuilderExt, execute_configured_build,
        },
    },
    configurers::{
        default_state_configurer::DefaultStateConfigurer, state_configurer::StateConfigurer,
    },
    model::{state_data::StateData, states_data::StatesData},
};

#[derive(Clone)]
pub struct StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    state_datas: Rc<RefCell<Vec<StateData<S, E>>>>,

    base: BaseConfiguredBuilder<
        StatesData<S, E>,
        Box<dyn StateMachineStateConfigurer<S, E>>,
        StateMachineStateBuilder<S, E>,
    >,
}

impl<S, E> StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        Self {
            state_datas: Rc::new(RefCell::new(Vec::new())),

            base: BaseConfiguredBuilder::with_allow_configurers_of_same_type(
                allow_configurers_of_same_type,
            ),
        }
    }

    pub fn add_state_data(&self, state_data: Vec<StateData<S, E>>) {
        self.state_datas.borrow_mut().extend(state_data);
    }
}

impl<S, E> BaseConfiguredBuilderExt<StatesData<S, E>> for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn perform_build(&mut self) -> Result<StatesData<S, E>, BoxError> {
        Ok(StatesData::new(self.state_datas.borrow().clone()))
    }
}

impl<S, E> BaseBuilderExt<StatesData<S, E>> for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn do_build(&mut self) -> Result<StatesData<S, E>, BoxError> {
        execute_configured_build(self)
    }
}

impl<S, E> AsRef<Self> for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<S, E> Required<BaseBuilder<StatesData<S, E>>> for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn get_object(&self) -> &BaseBuilder<StatesData<S, E>> {
        &self.base.base
    }

    fn get_mut_object(&mut self) -> &mut BaseBuilder<StatesData<S, E>> {
        &mut self.base.base
    }
}

impl<S, E> Deref for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    type Target =
        BaseConfiguredBuilder<StatesData<S, E>, Box<dyn StateMachineStateConfigurer<S, E>>, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<S, E> DerefMut for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<S, E>
    Required<
        BaseConfiguredBuilder<StatesData<S, E>, Box<dyn StateMachineStateConfigurer<S, E>>, Self>,
    > for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn get_object(
        &self,
    ) -> &BaseConfiguredBuilder<StatesData<S, E>, Box<dyn StateMachineStateConfigurer<S, E>>, Self>
    {
        &self.base
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseConfiguredBuilder<
        StatesData<S, E>,
        Box<dyn StateMachineStateConfigurer<S, E>>,
        Self,
    > {
        &mut self.base
    }
}

impl<S, E> StateMachineStateConfigurer<S, E> for StateMachineStateBuilder<S, E>
where
    S: Eq + Hash + Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    fn with_states(&mut self) -> Result<Box<dyn StateConfigurer<S, E>>, BoxError> {
        let mut state_configurer = DefaultStateConfigurer::default();
        self.base.apply(&mut state_configurer)?;

        Ok(Box::new(state_configurer))
    }
}

impl<S, E> Default for StateMachineStateBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            state_datas: Rc::new(RefCell::new(Vec::new())),

            base: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{
        builders::state_machine_state_configurer::StateMachineStateConfigurer,
        common::builder::Builder, configurers::state_configurer::StateConfigurer,
    };

    use super::StateMachineStateBuilder;

    #[test]
    fn with_states_registers_real_configurer() {
        let mut builder = StateMachineStateBuilder::<i32, i32>::new(true);
        builder.with_states().unwrap().initial(1).state(2);

        let states = builder.build().unwrap();
        let mut states = states.state_data().iter().collect::<Vec<_>>();
        states.sort_by_key(|state| *state.state());

        assert_eq!(states.len(), 2);
        assert_eq!(states[0].state(), &1);
        assert!(states[0].is_initial());
        assert_eq!(states[1].state(), &2);
    }
}
