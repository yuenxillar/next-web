use std::{any::Any, sync::Arc};

use crate::{
    BoxedStateAction, StateMachine,
    config::{action::StateMachineAction, state_machine_factory::StateMachineFactory},
    state::pseudo_state_kind::PseudoStateKind,
};

/// `StateData` is a data representation of a `State` used as an
/// abstraction between a `StateMachineFactory` and a state machine
/// configuration.
///
/// This struct encapsulates all the data needed to define a state
/// within a state machine, including its hierarchical structure,
/// actions, deferred events, and pseudo-state properties.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[derive(Clone)]
pub struct StateData<S, E> {
    /// Parent state reference in a hierarchical state machine
    pub(crate) parent: Option<Arc<dyn Any>>,
    /// Region identifier for parallel state machines
    pub(crate) region: Option<String>,
    /// The actual state value
    pub(crate) state: S,
    /// Collection of sub-state data for hierarchical states
    pub(crate) submachine_state_data: Option<Vec<StateData<S, E>>>,
    /// Sub-state machine instance
    submachine: Option<Arc<dyn StateMachine<S, E>>>,
    /// Factory for creating sub-state machines
    submachine_factory: Option<Arc<dyn StateMachineFactory<S, E>>>,
    /// Collection of events deferred in this state
    deferred: Option<Vec<E>>,
    /// Collection of entry action functions
    entry_actions: Option<Vec<BoxedStateAction<S, E>>>,
    /// Collection of exit action functions
    exit_actions: Option<Vec<BoxedStateAction<S, E>>>,
    /// Collection of state action functions
    state_actions: Option<Vec<BoxedStateAction<S, E>>>,
    /// Flag indicating if this is an initial state
    initial: bool,
    /// Action to execute when entering as initial state
    initial_action: Option<Arc<dyn StateMachineAction<S, E>>>,
    /// Flag indicating if this is an end (final) state
    end: bool,
    /// Kind of pseudo-state (choice, junction, fork, join, etc.)
    pseudo_state_kind: Option<PseudoStateKind>,
}

impl<S, E> StateData<S, E> {
    /// Creates a new state data with just the state value.
    ///
    /// # Arguments
    /// * `state` - The state value
    ///
    /// # Returns
    /// A new `StateData` instance with default values
    pub fn new(state: S) -> Self {
        Self::with_initial(state, false)
    }

    /// Creates a new state data with state value and initial flag.
    ///
    /// # Arguments
    /// * `state` - The state value
    /// * `initial` - Whether this state is initial
    ///
    /// # Returns
    /// A new `StateData` instance
    pub fn with_initial(state: S, initial: bool) -> Self {
        Self {
            parent: None,
            region: None,
            state,
            submachine_state_data: None,
            submachine: None,
            submachine_factory: None,
            deferred: None,
            entry_actions: None,
            exit_actions: None,
            state_actions: None,
            initial,
            initial_action: None,
            end: false,
            pseudo_state_kind: None,
        }
    }

    /// Creates a new state data with hierarchical context.
    ///
    /// # Arguments
    /// * `parent` - Parent state reference
    /// * `region` - Region identifier
    /// * `state` - The state value
    /// * `initial` - Whether this state is initial
    ///
    /// # Returns
    /// A new `StateData` instance
    pub fn with_hierarchy(
        parent: Option<Arc<dyn Any>>,
        region: Option<String>,
        state: S,
        initial: bool,
    ) -> Self {
        Self {
            parent,
            region,
            state,
            submachine_state_data: None,
            submachine: None,
            submachine_factory: None,
            deferred: None,
            entry_actions: None,
            exit_actions: None,
            state_actions: None,
            initial,
            initial_action: None,
            end: false,
            pseudo_state_kind: None,
        }
    }

    /// Creates a new state data with actions and deferred events.
    ///
    /// # Arguments
    /// * `parent` - Parent state reference
    /// * `region` - Region identifier
    /// * `state` - The state value
    /// * `deferred` - Collection of deferred events
    /// * `entry_actions` - Collection of entry action functions
    /// * `exit_actions` - Collection of exit action functions
    ///
    /// # Returns
    /// A new `StateData` instance
    pub fn with_actions(
        parent: Option<Arc<dyn Any>>,
        region: Option<String>,
        state: S,
        deferred: Option<Vec<E>>,
        entry_actions: Option<Vec<BoxedStateAction<S, E>>>,
        exit_actions: Option<Vec<BoxedStateAction<S, E>>>,
    ) -> Self {
        Self::with_actions_extended(
            parent,
            region,
            state,
            deferred,
            entry_actions,
            exit_actions,
            false,
            None,
        )
    }

    /// Creates a new state data with actions, deferred events, and initial properties.
    ///
    /// # Arguments
    /// * `parent` - Parent state reference
    /// * `region` - Region identifier
    /// * `state` - The state value
    /// * `deferred` - Collection of deferred events
    /// * `entry_actions` - Collection of entry action functions
    /// * `exit_actions` - Collection of exit action functions
    /// * `initial` - Whether this state is initial
    /// * `initial_action` - Action to execute when entering as initial state
    ///
    /// # Returns
    /// A new `StateData` instance
    pub fn with_actions_extended(
        parent: Option<Arc<dyn Any>>,
        region: Option<String>,
        state: S,
        deferred: Option<Vec<E>>,
        entry_actions: Option<Vec<BoxedStateAction<S, E>>>,
        exit_actions: Option<Vec<BoxedStateAction<S, E>>>,
        initial: bool,
        initial_action: Option<Arc<dyn StateMachineAction<S, E>>>,
    ) -> Self {
        Self {
            parent,
            region,
            state,
            submachine_state_data: None,
            submachine: None,
            submachine_factory: None,
            deferred,
            entry_actions,
            exit_actions,
            state_actions: None,
            initial,
            initial_action,
            end: false,
            pseudo_state_kind: None,
        }
    }

    /// Gets the state value.
    ///
    /// # Returns
    /// A reference to the state value
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Gets the submachine state data collection.
    ///
    /// # Returns
    /// A reference to the submachine state data if present
    pub fn submachine_state_data(&self) -> Option<&[StateData<S, E>]> {
        self.submachine_state_data.as_deref()
    }

    /// Sets the submachine state data.
    ///
    /// # Arguments
    /// * `submachine_state_data` - Collection of sub-state data
    pub fn set_submachine_state_data(&mut self, submachine_state_data: Vec<StateData<S, E>>) {
        self.submachine_state_data = Some(submachine_state_data);
    }

    /// Gets the submachine.
    ///
    /// # Returns
    /// A reference to the submachine if present
    pub fn submachine(&self) -> Option<&dyn StateMachine<S, E>> {
        self.submachine.as_deref()
    }

    /// Gets a cloned submachine handle if present.
    pub fn submachine_arc(&self) -> Option<Arc<dyn StateMachine<S, E>>> {
        self.submachine.clone()
    }

    /// Sets the submachine.
    ///
    /// # Arguments
    /// * `submachine` - Sub-state machine instance
    pub fn set_submachine(&mut self, submachine: Arc<dyn StateMachine<S, E>>) {
        self.submachine = Some(submachine);
    }

    /// Gets the submachine factory.
    ///
    /// # Returns
    /// A reference to the submachine factory if present
    pub fn submachine_factory(&self) -> Option<&dyn StateMachineFactory<S, E>> {
        self.submachine_factory.as_deref()
    }

    /// Gets a cloned submachine factory handle if present.
    pub fn submachine_factory_arc(&self) -> Option<Arc<dyn StateMachineFactory<S, E>>> {
        self.submachine_factory.clone()
    }

    /// Sets the submachine factory.
    ///
    /// # Arguments
    /// * `submachine_factory` - Factory for creating sub-state machines
    pub fn set_submachine_factory(
        &mut self,
        submachine_factory: Arc<dyn StateMachineFactory<S, E>>,
    ) {
        self.submachine_factory = Some(submachine_factory);
    }

    /// Gets the deferred events collection.
    ///
    /// # Returns
    /// A reference to the deferred events if present
    pub fn deferred(&self) -> Option<&[E]> {
        self.deferred.as_deref()
    }

    /// Sets the deferred events.
    ///
    /// # Arguments
    /// * `deferred` - Collection of deferred events
    pub fn set_deferred(&mut self, deferred: Vec<E>) {
        self.deferred = Some(deferred);
    }

    /// Gets the entry actions collection.
    ///
    /// # Returns
    /// A reference to the entry actions if present
    pub fn entry_actions(&self) -> Option<&[BoxedStateAction<S, E>]> {
        self.entry_actions.as_deref()
    }

    /// Sets the entry actions.
    ///
    /// # Arguments
    /// * `entry_actions` - Collection of entry action functions
    pub fn set_entry_actions(&mut self, entry_actions: Vec<BoxedStateAction<S, E>>) {
        self.entry_actions = Some(entry_actions);
    }

    /// Gets the exit actions collection.
    ///
    /// # Returns
    /// A reference to the exit actions if present
    pub fn exit_actions(&self) -> Option<&[BoxedStateAction<S, E>]> {
        self.exit_actions.as_deref()
    }

    /// Sets the exit actions.
    ///
    /// # Arguments
    /// * `exit_actions` - Collection of exit action functions
    pub fn set_exit_actions(&mut self, exit_actions: Vec<BoxedStateAction<S, E>>) {
        self.exit_actions = Some(exit_actions);
    }

    /// Gets the state actions collection.
    ///
    /// # Returns
    /// A reference to the state actions if present
    pub fn state_actions(&self) -> Option<&[BoxedStateAction<S, E>]> {
        self.state_actions.as_deref()
    }

    /// Sets the state actions.
    ///
    /// # Arguments
    /// * `state_actions` - Collection of state action functions
    pub fn set_state_actions(&mut self, state_actions: Vec<BoxedStateAction<S, E>>) {
        self.state_actions = Some(state_actions);
    }

    /// Gets the parent state reference.
    ///
    /// # Returns
    /// The parent reference if present
    pub fn parent(&self) -> Option<&dyn Any> {
        self.parent.as_deref()
    }

    /// Sets the parent state.
    ///
    /// # Arguments
    /// * `parent` - Parent state reference
    pub fn set_parent(&mut self, parent: Arc<dyn Any>) {
        self.parent = Some(parent);
    }

    /// Gets the region identifier.
    ///
    /// # Returns
    /// The region reference if present
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Sets the region identifier.
    ///
    /// # Arguments
    /// * `region` - Region identifier
    pub fn set_region(&mut self, region: impl Into<String>) {
        self.region = Some(region.into());
    }

    /// Checks if this state is initial.
    ///
    /// # Returns
    /// `true` if this is an initial state, `false` otherwise
    pub fn is_initial(&self) -> bool {
        self.initial
    }

    /// Sets the initial flag.
    ///
    /// # Arguments
    /// * `initial` - Whether this state is initial
    pub fn set_initial(&mut self, initial: bool) {
        self.initial = initial;
    }

    /// Gets the initial action.
    ///
    /// # Returns
    /// A reference to the initial action if present
    pub fn initial_action(&self) -> Option<&dyn StateMachineAction<S, E>> {
        self.initial_action.as_deref()
    }

    /// Gets a cloned initial action handle if present.
    pub fn initial_action_arc(&self) -> Option<Arc<dyn StateMachineAction<S, E>>> {
        self.initial_action.clone()
    }

    /// Sets the initial action.
    ///
    /// # Arguments
    /// * `initial_action` - Action to execute when entering as initial state
    pub fn set_initial_action(&mut self, initial_action: Arc<dyn StateMachineAction<S, E>>) {
        self.initial_action = Some(initial_action);
    }

    /// Checks if this state is an end (final) state.
    ///
    /// # Returns
    /// `true` if this is an end state, `false` otherwise
    pub fn is_end(&self) -> bool {
        self.end
    }

    /// Sets the end flag.
    ///
    /// # Arguments
    /// * `end` - Whether this state is an end state
    pub fn set_end(&mut self, end: bool) {
        self.end = end;
    }

    /// Gets the pseudo-state kind.
    ///
    /// # Returns
    /// The pseudo-state kind if present
    pub fn pseudo_state_kind(&self) -> Option<PseudoStateKind> {
        self.pseudo_state_kind
    }

    /// Sets the pseudo-state kind.
    ///
    /// # Arguments
    /// * `pseudo_state_kind` - Kind of pseudo-state
    pub fn set_pseudo_state_kind(&mut self, pseudo_state_kind: PseudoStateKind) {
        self.pseudo_state_kind = Some(pseudo_state_kind);
    }
}

impl<S, E> std::fmt::Debug for StateData<S, E>
where
    S: std::fmt::Debug,
    E: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StateData")
            .field("parent", &"...")
            .field(
                "region",
                &self
                    .region
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            )
            .field("state", &self.state)
            .field("deferred", &self.deferred)
            .field(
                "entry_actions",
                &self.entry_actions.as_ref().map(|v| v.len()),
            )
            .field("exit_actions", &self.exit_actions.as_ref().map(|v| v.len()))
            .field("initial", &self.initial)
            .field(
                "initial_action",
                &self.initial_action.as_ref().map(|_| "..."),
            )
            .field("end", &self.end)
            .field("pseudo_state_kind", &self.pseudo_state_kind)
            .finish()
    }
}
