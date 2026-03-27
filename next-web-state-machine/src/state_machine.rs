use std::any::Any;
use std::{collections::HashSet, fmt::Debug, hash::Hash, sync::Arc};

use next_web_core::BoxFuture;

use crate::access::state_machine_accessor::StateMachineAccessor;
use crate::config::state_machine_configure::StateMachineConfigure;
use crate::config::state_machine_state_configure::StateMachineStateConfigure;
use crate::config::state_machine_transition_configure::{
    ExternalTransitionConfigure, StateMachineTransitionConfigure,
};
use crate::extended_state::ExtendedState;
use crate::region::Region;
use crate::state::StateMachineState;
use crate::state_context::StateContext;

pub type BoxedStateAction<S, E> =
    Arc<dyn for<'a> Fn(&'a dyn StateContext<S, E>) -> BoxFuture<'a, ()> + Send + Sync>;
pub type BoxedStateGuard<S, E> =
    Arc<dyn for<'a> Fn(&'a dyn StateContext<S, E>) -> BoxFuture<'a, bool> + Send + Sync>;

pub trait StateMachine<S, E>
where
    Self: Send + Sync,
    Self: Any,
    Self: Region<S, E>,
{
    /// Gets the initial state S.
    fn initial_state(&self) -> &dyn StateMachineState<S, E>;

    /// Gets the state machine extended state.
    fn extended_state(&self) -> &dyn ExtendedState;

    fn state_machine_accessor(&self) -> &dyn StateMachineAccessor<S, E>;

    /// Sets the state machine error.
    fn set_state_machine_error(&mut self, error: Box<dyn std::error::Error + Send>);

    /// Checks for state machine error.
    fn has_state_machine_error(&self) -> bool;
}

pub struct DefaultStateMachine<S, E> {
    pub(crate) id: String,
    // todo
    pub(crate) configure: StateMachineConfigure<S, E>,
    pub(crate) state_configure: StateMachineStateConfigure<S, E>,
    pub(crate) transition_configure: StateMachineTransitionConfigure<S, E>,
    pub(crate) status: bool,
}

impl<S, E> DefaultStateMachine<S, E>
where
    E: Send + Sync + 'static,
    E: Clone + Debug + Hash + Eq,
    S: Send + Sync + 'static,
    S: Clone + Hash + Eq,
{
    pub fn from_configure(
        configure: StateMachineConfigure<S, E>,
        state_configure: StateMachineStateConfigure<S, E>,
        transition_configure: StateMachineTransitionConfigure<S, E>,
    ) -> Self {
        Self {
            id: String::default(),
            configure,
            state_configure,
            transition_configure,
            status: false,
        }
    }

    pub async fn start(mut self) -> Arc<Self> {
        self.status = true;

        let state_machine = Arc::new(self);

        state_machine
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn initial_state(&self) -> &S {
        &self.state_configure.initial_state()
    }

    pub fn states(&self) -> &HashSet<S> {
        &self.state_configure.states
    }

    pub fn transitions(&self) -> &Vec<ExternalTransitionConfigure<S, E>> {
        &self.transition_configure.inner
    }

    pub fn is_complete(&self) -> bool {
        false
    }

    pub fn set_id(mut self, id: impl ToString) -> Self {
        self.id = id.to_string();
        self
    }
}

impl<S, E> Default for DefaultStateMachine<S, E>
where
    S: Default,
    E: Default,
{
    fn default() -> Self {
        Self {
            id: Default::default(),
            configure: Default::default(),
            state_configure: Default::default(),
            transition_configure: Default::default(),
            status: Default::default(),
        }
    }
}

impl<S, E> Clone for DefaultStateMachine<S, E>
where
    S: Clone,
    E: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            configure: self.configure.clone(),
            state_configure: self.state_configure.clone(),
            transition_configure: self.transition_configure.clone(),
            status: self.status.clone(),
        }
    }
}
