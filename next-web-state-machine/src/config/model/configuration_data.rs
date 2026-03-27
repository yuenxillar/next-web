use std::{sync::Arc, time::Duration};

use crate::{
    action::state_do_action_policy::StateDoActionPolicy,
    config::model::verifier::{
        default_state_machine_model_verifier::DefaultStateMachineModelVerifier,
        state_machine_model_verifier::StateMachineModelVerifier,
    },
    ensemble::state_machine_ensemble::StateMachineEnsemble,
    listener::state_machine_listener::StateMachineListener,
    monitor::state_machine_monitor::StateMachineMonitor,
    region::region_execution_policy::RegionExecutionPolicy,
    security::{access_decision_manager::AccessDecisionManager, security_rule::SecurityRule},
    support::state_machine_interceptor::StateMachineInterceptor,
    transition::transition_conflict_policy::TransitionConflictPolicy,
};

/// Configuration object used to keep things together in StateMachineConfigurationBuilder.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Clone)]
pub struct ConfigurationData<S, E> {
    /// The machine identifier.
    pub(crate) machine_id: Option<String>,
    /// Auto-start flag.
    pub(crate) auto_start: bool,
    /// Transition conflict resolution policy.
    pub(crate) transition_conflict_policy: Option<TransitionConflictPolicy>,
    /// State do-action execution policy.
    pub(crate) state_do_action_policy: Option<StateDoActionPolicy>,
    /// Timeout for state do-action policy.
    pub(crate) state_do_action_policy_timeout: Option<Duration>,
    /// State machine ensemble for distributed coordination.
    pub(crate) ensemble: Option<Arc<dyn StateMachineEnsemble<S, E>>>,
    /// Collection of state machine lifecycle listeners.
    pub(crate) listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
    /// Security subsystem enabled flag.
    pub(crate) security_enabled: bool,
    /// Model verifier enabled flag.
    pub(crate) verifier_enabled: bool,
    /// State machine model validator/verifier.
    pub(crate) verifier: Option<Arc<dyn StateMachineModelVerifier<S, E>>>,
    /// Access decision manager for transition security checks.
    pub(crate) transition_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    /// Access decision manager for event security checks.
    pub(crate) event_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
    /// Security rule for event processing.
    pub(crate) event_security_rule: Option<SecurityRule>,
    /// Security rule for transition processing.
    pub(crate) transition_security_rule: Option<SecurityRule>,
    /// State machine performance/behavior monitor.
    pub(crate) state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
    /// Interceptors for state machine processing pipeline.
    pub(crate) interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>,
    /// Execution policy for parallel regions.
    pub(crate) region_execution_policy: Option<RegionExecutionPolicy>,
}

impl<S, E> ConfigurationData<S, E>
where
    S: 'static,
    E: 'static,
{
    /// Creates a new state machine configuration with specified parameters.
    ///
    /// # Arguments
    /// * `auto_start` - Whether to automatically start the state machine
    /// * `ensemble` - Distributed coordination ensemble
    /// * `listeners` - State machine lifecycle listeners
    /// * `security_enabled` - Enable security subsystem
    /// * `transition_security_access_decision_manager` - Security manager for transitions
    /// * `event_security_access_decision_manager` - Security manager for events
    /// * `event_security_rule` - Security rule for event processing
    /// * `transition_security_rule` - Security rule for transition processing
    /// * `verifier_enabled` - Enable model verification
    /// * `verifier` - Model verification implementation
    /// * `machine_id` - Unique identifier for the state machine
    /// * `state_machine_monitor` - Performance/behavior monitoring
    /// * `interceptors` - Processing pipeline interceptors
    ///
    /// # Returns
    /// A new `ConfigurationData` instance with the specified parameters
    pub fn with_parameters(
        auto_start: bool,
        ensemble: Option<Arc<dyn StateMachineEnsemble<S, E>>>,
        listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
        security_enabled: bool,
        transition_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
        event_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
        event_security_rule: Option<SecurityRule>,
        transition_security_rule: Option<SecurityRule>,
        verifier_enabled: bool,
        verifier: Option<Arc<dyn StateMachineModelVerifier<S, E>>>,
        machine_id: Option<String>,
        state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
        interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>,
    ) -> Self {
        Self {
            machine_id,
            auto_start,
            transition_conflict_policy: None,
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
            ensemble,
            listeners,
            security_enabled,
            verifier_enabled,
            verifier,
            transition_security_access_decision_manager,
            event_security_access_decision_manager,
            event_security_rule,
            transition_security_rule,
            state_machine_monitor,
            interceptors,
            region_execution_policy: None,
        }
    }

    /// Creates a new state machine configuration with all parameters.
    ///
    /// # Arguments
    /// * `bean_factory` - Dependency injection container
    /// * `auto_start` - Whether to automatically start the state machine
    /// * `ensemble` - Distributed coordination ensemble
    /// * `listeners` - State machine lifecycle listeners
    /// * `security_enabled` - Enable security subsystem
    /// * `transition_security_access_decision_manager` - Security manager for transitions
    /// * `event_security_access_decision_manager` - Security manager for events
    /// * `event_security_rule` - Security rule for event processing
    /// * `transition_security_rule` - Security rule for transition processing
    /// * `verifier_enabled` - Enable model verification
    /// * `verifier` - Model verification implementation
    /// * `machine_id` - Unique identifier for the state machine
    /// * `state_machine_monitor` - Performance/behavior monitoring
    /// * `interceptors` - Processing pipeline interceptors
    /// * `transition_conflict_policy` - Policy for resolving conflicting transitions
    /// * `state_do_action_policy` - Policy for executing state actions
    /// * `state_do_action_policy_timeout` - Timeout for state action execution
    /// * `region_execution_policy` - Policy for parallel region execution
    ///
    /// # Returns
    /// A new `ConfigurationData` instance with all specified parameters
    pub fn with_all_parameters(
        auto_start: bool,
        ensemble: Option<Arc<dyn StateMachineEnsemble<S, E>>>,
        listeners: Vec<Arc<dyn StateMachineListener<S, E>>>,
        security_enabled: bool,
        transition_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
        event_security_access_decision_manager: Option<Arc<dyn AccessDecisionManager>>,
        event_security_rule: Option<SecurityRule>,
        transition_security_rule: Option<SecurityRule>,
        verifier_enabled: bool,
        verifier: Option<Arc<dyn StateMachineModelVerifier<S, E>>>,
        machine_id: Option<String>,
        state_machine_monitor: Option<Arc<dyn StateMachineMonitor<S, E>>>,
        interceptors: Vec<Arc<dyn StateMachineInterceptor<S, E>>>,
        transition_conflict_policy: Option<TransitionConflictPolicy>,
        state_do_action_policy: Option<StateDoActionPolicy>,
        state_do_action_policy_timeout: Option<Duration>,
        region_execution_policy: Option<RegionExecutionPolicy>,
    ) -> Self {
        Self {
            machine_id,
            auto_start,
            transition_conflict_policy,
            state_do_action_policy,
            state_do_action_policy_timeout,
            ensemble,
            listeners,
            security_enabled,
            verifier_enabled,
            verifier,
            transition_security_access_decision_manager,
            event_security_access_decision_manager,
            event_security_rule,
            transition_security_rule,
            state_machine_monitor,
            interceptors,
            region_execution_policy,
        }
    }

    /// Gets the machine identifier.
    ///
    /// # Returns
    /// The machine identifier if set, otherwise None
    pub fn machine_id(&self) -> Option<&str> {
        self.machine_id.as_deref()
    }

    /// Checks if auto-start is enabled.
    ///
    /// # Returns
    /// `true` if auto-start is enabled, `false` otherwise
    pub fn is_auto_start(&self) -> bool {
        self.auto_start
    }

    /// Gets the transition conflict resolution policy.
    ///
    /// # Returns
    /// The transition conflict policy if set, otherwise None
    pub fn transition_conflict_policy(&self) -> Option<&TransitionConflictPolicy> {
        self.transition_conflict_policy.as_ref()
    }

    /// Gets the state do-action execution policy.
    ///
    /// # Returns
    /// The state do-action policy if set, otherwise None
    pub fn state_do_action_policy(&self) -> Option<&StateDoActionPolicy> {
        self.state_do_action_policy.as_ref()
    }

    /// Gets the timeout for state do-action policy.
    ///
    /// # Returns
    /// The state do-action policy timeout if set, otherwise None
    pub fn state_do_action_policy_timeout(&self) -> Option<Duration> {
        self.state_do_action_policy_timeout
    }

    /// Gets the state machine ensemble for distributed coordination.
    ///
    /// # Returns
    /// The state machine ensemble if set, otherwise None
    pub fn ensemble(&self) -> Option<&dyn StateMachineEnsemble<S, E>> {
        self.ensemble.as_deref()
    }

    /// Gets the collection of state machine listeners.
    ///
    /// # Returns
    /// A reference to the listeners collection
    pub fn listeners(&self) -> Vec<&dyn StateMachineListener<S, E>> {
        self.listeners.iter().map(AsRef::as_ref).collect()
    }

    /// Checks if security is enabled.
    ///
    /// # Returns
    /// `true` if security is enabled, `false` otherwise
    pub fn is_security_enabled(&self) -> bool {
        self.security_enabled
    }

    /// Checks if model verification is enabled.
    ///
    /// # Returns
    /// `true` if verifier is enabled, `false` otherwise
    pub fn is_verifier_enabled(&self) -> bool {
        self.verifier_enabled
    }

    /// Gets the state machine model verifier.
    ///
    /// # Returns
    /// The model verifier if set, otherwise None
    pub fn verifier(&self) -> Option<&dyn StateMachineModelVerifier<S, E>> {
        self.verifier.as_deref()
    }

    /// Gets the access decision manager for transition security.
    ///
    /// # Returns
    /// The transition security access decision manager if set, otherwise None
    pub fn transition_security_access_decision_manager(
        &self,
    ) -> Option<&dyn AccessDecisionManager> {
        self.transition_security_access_decision_manager.as_deref()
    }

    /// Gets the access decision manager for event security.
    ///
    /// # Returns
    /// The event security access decision manager if set, otherwise None
    pub fn event_security_access_decision_manager(&self) -> Option<&dyn AccessDecisionManager> {
        self.event_security_access_decision_manager.as_deref()
    }

    /// Gets the security rule for event processing.
    ///
    /// # Returns
    /// The event security rule if set, otherwise None
    pub fn event_security_rule(&self) -> Option<&SecurityRule> {
        self.event_security_rule.as_ref()
    }

    /// Gets the security rule for transition processing.
    ///
    /// # Returns
    /// The transition security rule if set, otherwise None
    pub fn transition_security_rule(&self) -> Option<&SecurityRule> {
        self.transition_security_rule.as_ref()
    }

    /// Gets the state machine monitor.
    ///
    /// # Returns
    /// The state machine monitor if set, otherwise None
    pub fn state_machine_monitor(&self) -> Option<&dyn StateMachineMonitor<S, E>> {
        self.state_machine_monitor.as_deref()
    }

    /// Gets the collection of state machine interceptors.
    ///
    /// # Returns
    /// A reference to the interceptors collection
    pub fn interceptors(&self) -> Vec<&dyn StateMachineInterceptor<S, E>> {
        self.interceptors.iter().map(AsRef::as_ref).collect()
    }

    /// Gets the region execution policy.
    ///
    /// # Returns
    /// The region execution policy if set, otherwise None
    pub fn region_execution_policy(&self) -> Option<&RegionExecutionPolicy> {
        self.region_execution_policy.as_ref()
    }
}

impl<S, E> Default for ConfigurationData<S, E>
where
    S: 'static,
    S: Send + Sync,
    E: 'static,
    E: Send + Sync,
{
    fn default() -> Self {
        Self {
            machine_id: None,
            auto_start: false,
            transition_conflict_policy: None,
            state_do_action_policy: None,
            state_do_action_policy_timeout: None,
            ensemble: None,
            listeners: Vec::new(),
            security_enabled: false,
            verifier_enabled: true,
            verifier: Some(Arc::new(DefaultStateMachineModelVerifier::<S, E>::default())),
            transition_security_access_decision_manager: None,
            event_security_access_decision_manager: None,
            event_security_rule: None,
            transition_security_rule: None,
            state_machine_monitor: None,
            interceptors: Vec::new(),
            region_execution_policy: None,
        }
    }
}
