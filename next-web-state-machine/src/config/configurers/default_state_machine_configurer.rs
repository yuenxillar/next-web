use std::time::Duration;
use std::{cell::RefCell, rc::Rc, sync::Arc};

use next_web_core::error::BoxError;

use crate::action::state_do_action_policy::StateDoActionPolicy;
use crate::config::builders::state_machine_configuration_builder::StateMachineConfigurationBuilder;
use crate::config::builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer;
use crate::config::common::configurer::Configurer;
use crate::config::common::configurer_adapter::{ConfigurerAdapter, ConfigurerAdapterExt};
use crate::config::configurer_builder::ConfigurerBuilder;
use crate::config::configurers::configuration_configurer::ConfigurationConfigurer;
use crate::config::model::configuration_data::ConfigurationData;
use crate::listener::state_machine_listener::StateMachineListener;
use crate::region::region_execution_policy::RegionExecutionPolicy;
use crate::transition::transition_conflict_policy::TransitionConflictPolicy;

/// Configuration data for state machine.
pub struct DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    shared: Rc<RefCell<DefaultConfigurationConfigurerData<S, E>>>,

    pub(crate) base: ConfigurerAdapter<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

struct DefaultConfigurationConfigurerData<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    machine_id: Option<String>,
    auto_start: bool,
    transition_conflict_policy: Option<TransitionConflictPolicy>,
    state_do_action_policy: Option<StateDoActionPolicy>,
    state_do_action_policy_timeout: Option<Duration>,
    region_execution_policy: Option<RegionExecutionPolicy>,
    listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineConfigurationConfigurer<S, E>>>
    for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineConfigurationConfigurer<S, E>> {
        todo!()
    }
}

impl<S, E> ConfigurationConfigurer<S, E> for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn machine_id(&mut self, id: String) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().machine_id = Some(id);
        self
    }

    fn auto_startup(&mut self, auto_startup: bool) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().auto_start = auto_startup;
        self
    }

    fn listener(
        &mut self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().listeners.push(listener);
        self
    }

    fn transition_conflict_policy(
        &mut self,
        transition_conflict_policy: TransitionConflictPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().transition_conflict_policy = Some(transition_conflict_policy);
        self
    }

    fn state_do_action_policy(
        &mut self,
        state_do_action_policy: StateDoActionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().state_do_action_policy = Some(state_do_action_policy);
        self
    }

    fn state_do_action_policy_timeout(
        &mut self,
        timeout: Duration,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().state_do_action_policy_timeout = Some(timeout);
        self
    }

    fn region_execution_policy(
        &mut self,
        region_execution_policy: RegionExecutionPolicy,
    ) -> &mut dyn ConfigurationConfigurer<S, E> {
        self.shared.borrow_mut().region_execution_policy = Some(region_execution_policy);
        self
    }
}

impl<S, E> DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn apply_to_builder(&self, builder: &StateMachineConfigurationBuilder<S, E>) {
        let data = self.shared.borrow();

        data.machine_id.as_ref().map(|id| {
            builder.set_machine_id(id.clone());
        });

        builder
            .set_auto_start(data.auto_start)
            .set_state_machine_listeners(data.listeners.clone());

        data.transition_conflict_policy.as_ref().map(|policy| {
            builder.set_transition_conflict_policy(policy.clone());
        });

        data.state_do_action_policy.as_ref().map(|policy| {
            builder.set_state_do_action_policy(
                policy.clone(),
                data.state_do_action_policy_timeout
                    .unwrap_or(Duration::from_millis(1)),
            );
        });

        data.region_execution_policy.as_ref().map(|policy| {
            builder.set_region_execution_policy(policy.clone());
        });
    }
}

impl<S, E> ConfigurerAdapterExt<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn configure(
        &mut self,
        builder: &mut StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        self.apply_to_builder(builder);
        Ok(())
    }
}

impl<S, E> Configurer<ConfigurationData<S, E>, StateMachineConfigurationBuilder<S, E>>
    for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn init(&mut self, _builder: &StateMachineConfigurationBuilder<S, E>) -> Result<(), BoxError> {
        Ok(())
    }

    fn configure(
        &mut self,
        builder: &StateMachineConfigurationBuilder<S, E>,
    ) -> Result<(), BoxError> {
        self.apply_to_builder(builder);
        Ok(())
    }

    fn is_assignable(&self, _builder: &StateMachineConfigurationBuilder<S, E>) -> bool {
        true
    }
}

impl<S, E> Default for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            shared: Rc::new(RefCell::new(DefaultConfigurationConfigurerData {
                machine_id: None,
                auto_start: false,
                transition_conflict_policy: None,
                state_do_action_policy: None,
                state_do_action_policy_timeout: None,
                region_execution_policy: None,
                listeners: Vec::new(),
            })),

            base: Default::default(),
        }
    }
}

impl<S, E> Clone for DefaultConfigurationConfigurer<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            base: self.base.clone(),
        }
    }
}
