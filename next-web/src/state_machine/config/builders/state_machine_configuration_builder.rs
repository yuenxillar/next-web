use next_web_core::error::BoxError;

use std::{any::Any, sync::Arc, time::Duration};

use crate::state_machine::{
    action::state_do_action_policy::StateDoActionPolicy,
    config::{
        builders::state_machine_configuration_configurer::StateMachineConfigurationConfigurer,
        common::{
            base_configured_builder::{BaseConfiguredBuilder, BaseConfiguredBuilderExt},
            builder::Builder,
            object_post_processor::ObjectPostProcessor,
        },
        configurers::{
            configuration_configurer::ConfigurationConfigurer,
            default_distributed_state_machine_configurer::DefaultDistributedStateMachineConfigurer,
            default_monitoring_configurer::DefaultMonitoringConfigurer,
            default_persistence_configurer::DefaultPersistenceConfigurer,
            default_security_configurer::DefaultSecurityConfigurer,
            default_state_machine_configurer::DefaultConfigurationConfigurer,
            default_verifier_configurer::DefaultVerifierConfigurer,
            distributed_state_machine_configurer::DistributedStateMachineConfigurer,
            monitoring_configurer::MonitoringConfigurer,
            persistence_configurer::PersistenceConfigurer,
            security_configurer::StateMachineSecurityConfigurer,
            verifier_configurer::StateMachineModelVerifierConfigurer,
        },
        model::{
            configuration_data::ConfigurationData,
            verifier::state_machine_model_verifier::StateMachineModelVerifier,
        },
    },
    ensemble::state_machine_ensemble::StateMachineEnsemble,
    listener::state_machine_listener::StateMachineListener,
    monitor::state_machine_monitor::StateMachineMonitor,
    persist::state_machine_runtime_persister::StateMachineRuntimePersister,
    region::region_execution_policy::RegionExecutionPolicy,
    security::{access_decision_manager::AccessDecisionManager, security_rule::SecurityRule},
    support::state_machine_interceptor::StateMachineInterceptor,
    transition::transition_conflict_policy::TransitionConflictPolicy,
};

/// Builder for state machine configuration.
///
/// Implements annotation builder pattern for constructing configuration data.
///
/// # Type Parameters
/// * `S` - Type representing states
/// * `E` - Type representing events
#[derive(Clone)]
pub struct StateMachineConfigurationBuilder<S, E> {
    /// Unique identifier for the state machine
    machine_id: Option<String>,
    /// Flag indicating if state machine should start automatically
    auto_start: bool,
    /// Policy for handling transition conflicts
    transition_conflict_policy: Option<TransitionConflictPolicy>,
    /// Policy for state action execution
    state_do_action_policy: Option<StateDoActionPolicy>,
    /// Timeout for state action policy
    state_do_action_policy_timeout: Option<Duration>,
    /// Execution policy for regions (parallel/sequential)
    region_execution_policy: Option<RegionExecutionPolicy>,
    /// Ensemble for distributed state machines
    ensemble: Option<Arc<dyn StateMachineEnsemble<S, E>>>,
    /// Collection of state machine listeners
    listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
    /// Flag indicating if security features are enabled
    security_enabled: bool,
    /// Flag indicating if model verification is enabled
    verifier_enabled: bool,
    /// Model verifier for state machine validation
    verifier: Option<Arc<dyn StateMachineModelVerifier<S, E>>>,
    /// Access decision manager for transition security
    transition_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    /// Access decision manager for event security
    event_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    /// Security rule for event authorization
    event_security_rule: Option<SecurityRule>,
    /// Security rule for transition authorization
    transition_security_rule: Option<SecurityRule>,
    /// Monitor for state machine metrics and monitoring
    state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
    /// Collection of state machine interceptors for cross-cutting concerns
    interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>,
    /// Persister for state machine runtime state persistence
    persister: Option<Arc<dyn StateMachineRuntimePersister<S, E, Box<dyn Any>>>>,

    base: BaseConfiguredBuilder<
        ConfigurationData<S, E>,
        Box<dyn StateMachineConfigurationConfigurer<S, E>>,
        StateMachineConfigurationBuilder<S, E>,
    >,
}

impl<S, E> StateMachineConfigurationBuilder<S, E> {
    pub fn new(allow_configurers_of_same_type: bool) -> Self {
        Self::_default(BaseConfiguredBuilder::with_allow_configurers_of_same_type(
            allow_configurers_of_same_type,
        ))
    }

    fn _default(
        base: BaseConfiguredBuilder<
            ConfigurationData<S, E>,
            Box<dyn StateMachineConfigurationConfigurer<S, E>>,
            StateMachineConfigurationBuilder<S, E>,
        >,
    ) -> Self {
        Self {
            machine_id: None,
            auto_start: false,
            transition_conflict_policy: None,
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
            region_execution_policy: None,
            ensemble: None,
            listeners: Vec::new(),
            security_enabled: false,
            verifier_enabled: true,
            verifier: None,
            transition_security_access_decision_manager: None,
            event_security_access_decision_manager: None,
            event_security_rule: None,
            transition_security_rule: None,
            state_machine_monitor: None,
            interceptors: Vec::new(),
            persister: None,
            base,
        }
    }

    /// Creates a new state machine configuration builder with object post-processor
    /// and configurer type allowance.
    ///
    /// # Arguments
    /// * `object_post_processor` - Processor for custom object initialization
    /// * `allow_configurers_of_same_type` - Flag allowing multiple same-type configurers
    pub fn with_object_post_processor_and_allow_configurers<T>(
        object_post_processor: T,
        allow_configurers_of_same_type: bool,
    ) -> Self
    where
        T: ObjectPostProcessor,
    {
        Self::_default(
            BaseConfiguredBuilder::with_post_processor_and_allow_same_type(
                object_post_processor,
                allow_configurers_of_same_type,
            ),
        )
    }

    /// Creates a new state machine configuration builder with object post-processor.
    ///
    /// # Arguments
    /// * `object_post_processor` - Processor for custom object initialization
    pub fn with_object_post_processor<T>(object_post_processor: T) -> Self
    where
        T: ObjectPostProcessor,
    {
        Self::_default(BaseConfiguredBuilder::with_post_processor(
            object_post_processor,
        ))
    }

    /// Sets the machine identifier.
    ///
    /// # Arguments
    /// * `machine_id` - Unique identifier for the state machine
    pub fn set_machine_id(&mut self, machine_id: impl Into<String>) -> &mut Self {
        self.machine_id = Some(machine_id.into());
        self
    }

    /// Sets the security enabled flag.
    ///
    /// # Arguments
    /// * `enabled` - If `true`, security is enabled for the state machine
    pub fn set_security_enabled(&mut self, enabled: bool) -> &mut Self {
        self.security_enabled = enabled;
        self
    }

    /// Sets the state machine ensemble for distributed coordination.
    ///
    /// # Arguments
    /// * `ensemble` - Ensemble for distributed state machine management
    pub fn set_state_machine_ensemble(
        &mut self,
        ensemble: Arc<dyn StateMachineEnsemble<S, E>>,
    ) -> &mut Self {
        self.ensemble = Some(ensemble);
        self
    }

    /// Sets the auto-start flag.
    ///
    /// # Arguments
    /// * `auto_start` - If `true`, state machine starts automatically after configuration
    pub fn set_auto_start(&mut self, auto_start: bool) -> &mut Self {
        self.auto_start = auto_start;
        self
    }

    /// Sets state machine listeners.
    ///
    /// # Arguments
    /// * `listeners` - Collection of listeners for state machine events
    pub fn set_state_machine_listeners(
        &mut self,
        listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
    ) -> &mut Self {
        self.listeners = listeners;
        self
    }

    /// Enables or disables model verification.
    ///
    /// # Arguments
    /// * `verifier_enabled` - If `true`, model verification is enabled
    pub fn set_verifier_enabled(&mut self, verifier_enabled: bool) -> &mut Self {
        self.verifier_enabled = verifier_enabled;
        self
    }

    /// Sets the state machine monitor for metrics collection.
    ///
    /// # Arguments
    /// * `state_machine_monitor` - Monitor for collecting state machine metrics
    pub fn set_state_machine_monitor(
        &mut self,
        state_machine_monitor: Arc<dyn StateMachineMonitor<S, E>>,
    ) -> &mut Self {
        self.state_machine_monitor = Some(state_machine_monitor);
        self
    }

    /// Sets the transition security access decision manager.
    ///
    /// # Arguments
    /// * `transition_security_access_decision_manager` - Manager for transition authorization decisions
    pub fn set_transition_security_access_decision_manager(
        &mut self,
        transition_security_access_decision_manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut Self {
        self.transition_security_access_decision_manager =
            Some(transition_security_access_decision_manager);
        self
    }

    /// Sets the event security access decision manager.
    ///
    /// # Arguments
    /// * `event_security_access_decision_manager` - Manager for event authorization decisions
    pub fn set_event_security_access_decision_manager(
        &mut self,
        event_security_access_decision_manager: Arc<dyn AccessDecisionManager>,
    ) -> &mut Self {
        self.event_security_access_decision_manager = Some(event_security_access_decision_manager);
        self
    }

    /// Sets the event security rule.
    ///
    /// # Arguments
    /// * `event_security_rule` - Rule defining event authorization policies
    pub fn set_event_security_rule(&mut self, event_security_rule: SecurityRule) -> &mut Self {
        self.event_security_rule = Some(event_security_rule);
        self
    }

    /// Sets the transition security rule.
    ///
    /// # Arguments
    /// * `transition_security_rule` - Rule defining transition authorization policies
    pub fn set_transition_security_rule(
        &mut self,
        transition_security_rule: SecurityRule,
    ) -> &mut Self {
        self.transition_security_rule = Some(transition_security_rule);
        self
    }

    /// Sets the state machine model verifier.
    ///
    /// # Arguments
    /// * `verifier` - Verifier for validating state machine model correctness
    pub fn set_verifier(
        &mut self,
        verifier: Arc<dyn StateMachineModelVerifier<S, E>>,
    ) -> &mut Self {
        self.verifier = Some(verifier);
        self
    }

    /// Sets the state machine runtime persister.
    ///
    /// # Arguments
    /// * `persister` - Persister for saving and restoring runtime state
    pub fn set_state_machine_runtime_persister(
        &mut self,
        persister: Arc<dyn StateMachineRuntimePersister<S, E, Box<dyn Any>>>,
    ) -> &mut Self {
        self.persister = Some(persister);
        self
    }

    /// Sets the transition conflict policy.
    ///
    /// # Arguments
    /// * `transition_conflict_policy` - Policy for resolving conflicting transitions
    pub fn set_transition_conflict_policy(
        &mut self,
        transition_conflict_policy: TransitionConflictPolicy,
    ) -> &mut Self {
        self.transition_conflict_policy = Some(transition_conflict_policy);
        self
    }

    /// Sets the state action execution policy with timeout.
    ///
    /// # Arguments
    /// * `state_do_action_policy` - Policy for executing state actions
    /// * `state_do_action_policy_timeout` - Timeout for state action execution
    pub fn set_state_do_action_policy(
        &mut self,
        state_do_action_policy: StateDoActionPolicy,
        state_do_action_policy_timeout: Duration,
    ) -> &mut Self {
        self.state_do_action_policy = Some(state_do_action_policy);
        self.state_do_action_policy_timeout = Some(state_do_action_policy_timeout);
        self
    }

    /// Sets the region execution policy.
    ///
    /// # Arguments
    /// * `region_execution_policy` - Policy for executing regions (parallel/sequential)
    pub fn set_region_execution_policy(
        &mut self,
        region_execution_policy: RegionExecutionPolicy,
    ) -> &mut Self {
        self.region_execution_policy = Some(region_execution_policy);
        self
    }
}

impl<S, E> Builder<ConfigurationData<S, E>> for StateMachineConfigurationBuilder<S, E> {
    fn build(&mut self) -> Result<ConfigurationData<S, E>, BoxError> {
        todo!()
    }
}

impl<S, E> StateMachineConfigurationConfigurer<S, E> for StateMachineConfigurationBuilder<S, E>
where
    S: 'static,
    S: Clone,
    E: 'static,
    E: Clone,
{
    fn with_configuration(&mut self) -> Result<Box<dyn ConfigurationConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultConfigurationConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }

    fn with_distributed(
        &mut self,
    ) -> Result<Box<dyn DistributedStateMachineConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultDistributedStateMachineConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }

    fn with_security(&mut self) -> Result<Box<dyn StateMachineSecurityConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultSecurityConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }

    fn with_verifier(
        &mut self,
    ) -> Result<Box<dyn StateMachineModelVerifierConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultVerifierConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }

    fn with_monitoring(&mut self) -> Result<Box<dyn MonitoringConfigurer<S, E>>, BoxError> {
        let mut configurer = DefaultMonitoringConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }

    fn with_persistence(
        &mut self,
    ) -> Result<Box<dyn PersistenceConfigurer<S, E, Box<dyn Any>>>, BoxError> {
        let mut configurer = DefaultPersistenceConfigurer::default();
        self.base.apply_adapter(&mut configurer.base);

        Ok(Box::new(configurer))
    }
}

impl<S, E> BaseConfiguredBuilderExt<ConfigurationData<S, E>>
    for StateMachineConfigurationBuilder<S, E>
{
    fn perform_build(&mut self) -> Result<ConfigurationData<S, E>, BoxError> {
        let mut interceptors = Vec::new();
        interceptors.extend(self.interceptors.clone());

        self.persister
            .as_ref()
            .map(|persister| persister.get_interceptor())
            .filter(|interceptor| interceptor.is_some())
            .map(|interceptor| interceptors.push(interceptor.unwrap()));

        let configuration_data = ConfigurationData {
            machine_id: self.machine_id.clone(),
            auto_start: self.auto_start,
            transition_conflict_policy: self.transition_conflict_policy.clone(),
            state_do_action_policy: self.state_do_action_policy.clone(),
            state_do_action_policy_timeout: self.state_do_action_policy_timeout.clone(),
            ensemble: self.ensemble.clone(),
            listeners: self.listeners.clone(),
            security_enabled: self.security_enabled,
            verifier_enabled: self.verifier_enabled,
            verifier: self.verifier.clone(),
            transition_security_access_decision_manager: self
                .transition_security_access_decision_manager
                .clone(),
            event_security_access_decision_manager: self
                .event_security_access_decision_manager
                .clone(),
            event_security_rule: self.event_security_rule.clone(),
            transition_security_rule: self.transition_security_rule.clone(),
            state_machine_monitor: self.state_machine_monitor.clone(),
            interceptors,
            region_execution_policy: self.region_execution_policy.clone(),
        };

        Ok(configuration_data)
    }
}

impl<S, E> Default for StateMachineConfigurationBuilder<S, E>
where
    S: Clone + 'static,
    E: Clone + 'static,
{
    fn default() -> Self {
        Self {
            machine_id: None,
            auto_start: false,
            transition_conflict_policy: None,
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
            region_execution_policy: None,
            ensemble: None,
            listeners: Vec::new(),
            security_enabled: false,
            verifier_enabled: true,
            verifier: None,
            transition_security_access_decision_manager: None,
            event_security_access_decision_manager: None,
            event_security_rule: None,
            transition_security_rule: None,
            state_machine_monitor: None,
            interceptors: Vec::new(),
            persister: None,
            base: Default::default(),
        }
    }
}
