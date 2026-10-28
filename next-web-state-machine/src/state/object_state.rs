//! The default state implementation.

use std::sync::Arc;

use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use tracing::warn;

use crate::BoxedStateAction;
use crate::action::state_do_action_policy::StateDoActionPolicy;
use crate::region::Region;
use crate::state::StateMachineState;
use crate::state::action_listener::ActionListener;
use crate::state::base_state::AbstractState;
use crate::state::pseudo_state::PseudoState;
use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::state_machine::StateMachine;
use crate::state_machine_event_result::{DefaultStateMachineEventResult, StateMachineEventResult};
use crate::support::state_machine_utils::StateMachineUtils;
use crate::transition::transition_kind::TransitionKind;

/// The default state implementation.
///
/// It folds the Java `AbstractSimpleState`, `ObjectState`, `EnumState` and
/// `StateMachineState` classes into one type: Rust has no implementation
/// inheritance and one concrete state is enough for every model, because `S`
/// is a single generic identifier type.
///
/// A state can additionally refer to a submachine or to several regions; those
/// are started on entry and stopped on exit, exactly like the Java classes do.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct ObjectState<S, E> {
    base: AbstractState<S, E>,
}

impl<S, E> ObjectState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    /// Creates a new object state.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    /// * `pseudo_state` - the pseudo state attached to this state
    pub fn new(id: S, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>) -> Self {
        Self {
            base: AbstractState::new(id, pseudo_state),
        }
    }

    /// Gets the shared state behaviour.
    pub fn base(&self) -> &AbstractState<S, E> {
        &self.base
    }

    /// Gets the shared state behaviour mutably.
    pub fn base_mut(&mut self) -> &mut AbstractState<S, E> {
        &mut self.base
    }

    /// Sets the deferred events of this state.
    ///
    /// # Arguments
    /// * `deferred` - the events deferred in this state
    pub fn with_deferred(mut self, deferred: Vec<E>) -> Self {
        self.base = self.base.with_deferred(deferred);
        self
    }

    /// Sets the entry actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed when the state is entered
    pub fn with_entry_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.base = self.base.with_entry_actions(actions);
        self
    }

    /// Sets the exit actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed when the state is exited
    pub fn with_exit_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.base = self.base.with_exit_actions(actions);
        self
    }

    /// Sets the state actions of this state.
    ///
    /// # Arguments
    /// * `actions` - the actions executed once, right after entry
    pub fn with_state_actions(mut self, actions: Vec<BoxedStateAction<S, E>>) -> Self {
        self.base = self.base.with_state_actions(actions);
        self
    }

    /// Sets the policy which decides how the state actions are executed.
    ///
    /// # Arguments
    /// * `policy` - the policy applied to the state actions
    /// * `timeout` - the timeout used by the timeout policy
    pub fn with_state_do_action_policy(
        mut self,
        policy: Option<StateDoActionPolicy>,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        self.base = self.base.with_state_do_action_policy(policy, timeout);
        self
    }

    /// Sets the regions of this state.
    ///
    /// # Arguments
    /// * `regions` - the orthogonal regions of this state
    pub fn with_regions(mut self, regions: Vec<Arc<dyn Region<S, E>>>) -> Self {
        self.base = self.base.with_regions(regions);
        self
    }

    /// Sets the submachine of this state.
    ///
    /// # Arguments
    /// * `submachine` - the submachine this state refers to
    pub fn with_submachine(mut self, submachine: Arc<dyn StateMachine<S, E>>) -> Self {
        self.base = self.base.with_submachine(submachine);
        self
    }

    /// Checks whether the transition targets this state and is local, in which
    /// case entry and exit actions must not run.
    ///
    /// # Arguments
    /// * `context` - the state context
    fn is_local(&self, context: &dyn StateContext<S, E>) -> bool
    where
        S: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        match context.transition() {
            Some(transition) => {
                transition.kind() == TransitionKind::Local
                    && transition
                        .target()
                        .id()
                        .eq(self.base.id())
            }
            None => false,
        }
    }
}

#[async_trait]
impl<S, E> StateMachineState<S, E> for ObjectState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    async fn send_event(
        &self,
        event: Arc<dyn Message<E>>,
    ) -> Vec<Arc<dyn StateMachineEventResult<S, E>>> {
        if let Some(submachine) = self.base.submachine() {
            return match submachine.send_event(event.clone()).await {
                Ok(result) => vec![result],
                Err(error) => {
                    warn!(%error, "submachine denied an event");
                    vec![Arc::new(DefaultStateMachineEventResult::denied(None, event))]
                }
            };
        }

        // An orthogonal state forwards the event to every region and reports
        // the results of all of them.
        let mut results: Vec<Arc<dyn StateMachineEventResult<S, E>>> = Vec::new();
        for region in self.base.regions() {
            match region.send_event(event.clone()).await {
                Ok(result) => results.push(result),
                Err(error) => {
                    warn!(%error, "region denied an event");
                }
            }
        }

        results
    }

    fn should_defer(&self, event: &dyn Message<E>) -> bool {
        if let Some(submachine) = self.base.submachine()
            && let Some(state) = submachine.state()
                && let Some(payload) = event.get_payload()
                    && state.deferred_events().iter().any(|item| item == payload) {
                        return true;
                    }

        self.base.should_defer_message(event)
    }

    async fn exit(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        let local = self.is_local(context);

        self.base.state_listener().on_exit(context).await;
        self.base.disarm_triggers();

        if !local {
            let actions: Vec<BoxedStateAction<S, E>> = self.base.exit_actions().to_vec();
            self.base.execute_actions(&actions, context).await?;
        }

        if let Some(submachine) = self.base.submachine()
            && should_stop(submachine.as_ref(), context) {
                submachine.stop().await?;
            }

        for region in self.base.regions() {
            region.stop().await?;
        }

        Ok(())
    }

    async fn entry(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        let local = self.is_local(context);

        self.base.state_listener().on_entry(context).await;
        self.base.arm_triggers();

        if !local {
            let actions: Vec<BoxedStateAction<S, E>> = self.base.entry_actions().to_vec();
            self.base.execute_actions(&actions, context).await?;
        }

        if let Some(submachine) = self.base.submachine() {
            if let Some(message) = context.message_arc() {
                submachine.set_forwarded_initial_event(message);
            }
            submachine.start().await?;
        }

        for region in self.base.regions() {
            region.start().await?;
        }

        self.base.schedule_state_actions(context).await;

        Ok(())
    }

    fn id(&self) -> &S {
        self.base.id()
    }

    fn ids(&self) -> &[S] {
        self.base.ids()
    }

    fn substates(&self) -> Vec<Arc<dyn StateMachineState<S, E>>> {
        // Hierarchy queries are answered by the state machine index built at
        // start up, which also covers the states of nested regions and
        // submachines. A state alone cannot rebuild that index.
        Vec::new()
    }

    fn contains_id(&self, id: &S) -> bool {
        if self.base.ids().iter().any(|item| item == id) {
            return true;
        }

        for region in self.base.regions() {
            if let Some(current) = region.state()
                && current.contains_id(id) {
                    return true;
                }
        }

        if let Some(submachine) = self.base.submachine()
            && let Some(current) = submachine.state()
                && current.contains_id(id) {
                    return true;
                }

        false
    }

    fn pseudo_state(&self) -> Option<Arc<dyn PseudoState<S, E>>> {
        self.base.pseudo_state()
    }

    fn set_pseudo_state(&self, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>) {
        self.base.set_pseudo_state(pseudo_state);
    }

    fn deferred_events(&self) -> &[E] {
        self.base.deferred_events()
    }

    fn add_trigger(&self, trigger: Arc<dyn crate::trigger::Trigger<S, E>>) {
        self.base.add_trigger(trigger);
    }

    fn arm_triggers(&self) {
        self.base.arm_triggers();
    }

    fn disarm_triggers(&self) {
        self.base.disarm_triggers();
    }

    fn entry_actions(&self) -> &[BoxedStateAction<S, E>] {
        self.base.entry_actions()
    }

    fn state_actions(&self) -> &[BoxedStateAction<S, E>] {
        self.base.state_actions()
    }

    fn exit_actions(&self) -> &[BoxedStateAction<S, E>] {
        self.base.exit_actions()
    }

    fn regions(&self) -> &[Arc<dyn Region<S, E>>] {
        self.base.regions()
    }

    fn submachine(&self) -> Option<&Arc<dyn StateMachine<S, E>>> {
        self.base.submachine()
    }

    fn add_state_listener(&self, listener: Arc<dyn StateListener<S, E>>) {
        self.base.add_listener(listener);
    }

    fn remove_state_listener(&self, listener: Arc<dyn StateListener<S, E>>) {
        self.base.remove_listener(listener);
    }

    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.base.add_action_listener_ref(listener);
    }

    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>) {
        self.base.remove_action_listener_ref(listener);
    }
}

/// Decides whether a submachine must be stopped when the state is exited.
///
/// The Java implementation keeps the submachine running when the transition
/// stays inside of it, so a restart does not re-enable the initial transition.
///
/// # Arguments
/// * `submachine` - the submachine of the state
/// * `context` - the state context
fn should_stop<S, E>(
    _submachine: &dyn StateMachine<S, E>,
    context: &dyn StateContext<S, E>,
) -> bool
where
    S: PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    let Some(transition) = context.transition() else {
        return true;
    };

    let Some(source) = transition.source() else {
        return true;
    };

    // Keep the submachine running while the transition moves to a descendant of
    // the source, otherwise the submachine is left.
    !StateMachineUtils::is_substate(&source, &transition.target())
}
