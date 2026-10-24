//! Default implementation of a state machine context.

use std::collections::HashMap;
use std::sync::Arc;

use crate::extended_state::ExtendedState;
use crate::state_machine_context::StateMachineContext;
use crate::support::default_extended_state::DefaultExtendedState;

/// Default implementation of [`StateMachineContext`].
///
/// The context captures everything needed to restore a machine: its identifier,
/// the current state, the triggering event, the history states and the extended
/// state variables.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct DefaultStateMachineContext<S, E> {
    id: String,
    childs: Vec<Box<dyn StateMachineContext<S, E>>>,
    child_references: Vec<String>,
    state: S,
    event: Option<E>,
    history_states: HashMap<S, S>,
    event_headers: HashMap<String, String>,
    extended_state: Arc<dyn ExtendedState>,
}

impl<S, E> DefaultStateMachineContext<S, E> {
    /// Creates a new context.
    ///
    /// # Arguments
    /// * `id` - the machine id
    /// * `state` - the current state
    pub fn new(id: impl Into<String>, state: S) -> Self {
        Self {
            id: id.into(),
            childs: Vec::new(),
            child_references: Vec::new(),
            state,
            event: None,
            history_states: HashMap::new(),
            event_headers: HashMap::new(),
            extended_state: Arc::new(DefaultExtendedState::default()),
        }
    }

    /// Sets the event which is being processed.
    ///
    /// # Arguments
    /// * `event` - the event
    pub fn with_event(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    /// Sets the history states.
    ///
    /// # Arguments
    /// * `history_states` - the mapping of a history state to the state it recorded
    pub fn with_history_states(mut self, history_states: HashMap<S, S>) -> Self {
        self.history_states = history_states;
        self
    }

    /// Sets the event headers.
    ///
    /// # Arguments
    /// * `event_headers` - the headers of the event which is being processed
    pub fn with_event_headers(mut self, event_headers: HashMap<String, String>) -> Self {
        self.event_headers = event_headers;
        self
    }

    /// Sets the extended state.
    ///
    /// # Arguments
    /// * `extended_state` - the extended state of the machine
    pub fn with_extended_state(mut self, extended_state: Arc<dyn ExtendedState>) -> Self {
        self.extended_state = extended_state;
        self
    }

    /// Sets the child contexts.
    ///
    /// # Arguments
    /// * `childs` - the contexts of the nested machines
    pub fn with_childs(mut self, childs: Vec<Box<dyn StateMachineContext<S, E>>>) -> Self {
        self.childs = childs;
        self
    }

    /// Sets the child context references.
    ///
    /// # Arguments
    /// * `child_references` - the identifiers of the nested machines
    pub fn with_child_references(mut self, child_references: Vec<String>) -> Self {
        self.child_references = child_references;
        self
    }
}

impl<S, E> StateMachineContext<S, E> for DefaultStateMachineContext<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn id(&self) -> &str {
        &self.id
    }

    fn childs(&self) -> &[Box<dyn StateMachineContext<S, E>>] {
        &self.childs
    }

    fn child_references(&self) -> &[String] {
        &self.child_references
    }

    fn state(&self) -> &S {
        &self.state
    }

    fn event(&self) -> Option<&E> {
        self.event.as_ref()
    }

    fn history_states(&self) -> &HashMap<S, S> {
        &self.history_states
    }

    fn event_headers(&self) -> &HashMap<String, String> {
        &self.event_headers
    }

    fn extended_state(&self) -> &dyn ExtendedState {
        self.extended_state.as_ref()
    }
}
