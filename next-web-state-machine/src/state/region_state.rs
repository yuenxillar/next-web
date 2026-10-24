//! A state which is backed by a region.

use std::sync::Arc;

use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::messaging::Message;

use crate::BoxedStateAction;
use crate::region::Region;
use crate::state::StateMachineState;
use crate::state::action_listener::ActionListener;
use crate::state::base_state::AbstractState;
use crate::state::pseudo_state::PseudoState;
use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::state_machine::StateMachine;
use crate::state_machine_event_result::StateMachineEventResult;

/// A state that delegates to a [`Region`].
///
/// The Java `RegionState` wraps one region and is used for the states of an
/// orthogonal state. Entry starts the region, exit stops it, and the ids and
/// states of the region are reported through the region.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct RegionState<S, E> {
    base: AbstractState<S, E>,
    region: Arc<dyn Region<S, E>>,
}

impl<S, E> RegionState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    /// Creates a new region state.
    ///
    /// # Arguments
    /// * `id` - the state identifier
    /// * `region` - the region behind this state
    pub fn new(id: S, region: Arc<dyn Region<S, E>>) -> Self {
        Self {
            base: AbstractState::new(id, None),
            region,
        }
    }

    /// Gets the region behind this state.
    pub fn region(&self) -> &Arc<dyn Region<S, E>> {
        &self.region
    }

    /// Gets the shared state behaviour.
    pub fn base(&self) -> &AbstractState<S, E> {
        &self.base
    }
}

#[async_trait]
impl<S, E> StateMachineState<S, E> for RegionState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Clone + PartialEq + Send + Sync + 'static,
{
    async fn send_event(
        &self,
        event: Arc<dyn Message<E>>,
    ) -> Vec<Arc<dyn StateMachineEventResult<S, E>>> {
        match self.region.send_event(event).await {
            Ok(result) => vec![result],
            Err(error) => {
                tracing::warn!(%error, "region denied an event");
                Vec::new()
            }
        }
    }

    fn should_defer(&self, event: &dyn Message<E>) -> bool {
        self.base.should_defer_message(event)
    }

    async fn exit(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        self.base.state_listener().on_exit(context).await;
        self.base.disarm_triggers();
        self.region.stop().await?;
        Ok(())
    }

    async fn entry(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError> {
        self.base.state_listener().on_entry(context).await;
        self.base.arm_triggers();
        if let Some(message) = context.message_arc() {
            self.region.set_forwarded_initial_event(message);
        }
        self.region.start().await?;
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
        Vec::new()
    }

    fn contains_id(&self, id: &S) -> bool {
        if self.base.ids().iter().any(|item| item == id) {
            return true;
        }

        match self.region.state() {
            Some(current) => current.contains_id(id),
            None => false,
        }
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
        std::slice::from_ref(&self.region)
    }

    fn submachine(&self) -> Option<&Arc<dyn StateMachine<S, E>>> {
        None
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
