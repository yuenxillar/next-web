use std::any::Any;
use std::{collections::HashSet, fmt::Debug, hash::Hash, sync::Arc};

use futures::future::BoxFuture;
use next_web_core::async_trait;
use next_web_core::{anys::any_value::AnyValue, DynClone};
use tokio::sync::broadcast::Sender;
use tracing::error;

use crate::state_machine::access::state_machine_accessor::StateMachineAccessor;
use crate::state_machine::config::state_machine_configure::StateMachineConfigure;
use crate::state_machine::config::state_machine_state_configure::StateMachineStateConfigure;
use crate::state_machine::config::state_machine_transition_configure::{
    ExternalTransitionConfigure, StateMachineTransitionConfigure,
};
use crate::state_machine::extended_state::ExtendedState;
use crate::state_machine::region::Region;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;
use crate::state_machine::state_machine_manager::StateMachineManager;

pub type BoxedStateAction<S, E> =
    Arc<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()> + Send + Sync>;
pub type BoxedStateGuard<S, E> =
    Arc<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, bool> + Send + Sync>;

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
    pub(crate) listener: Option<Box<dyn DefaultStateMachineListener<S, E>>>,
    pub(crate) sender: Option<Sender<EventMessage<E>>>,
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
            listener: None,
            sender: None,
            status: false,
        }
    }

    pub async fn start(mut self) -> Arc<Self> {
        // create manager
        let mut manager = StateMachineManager::<S, E>::new(self.id.clone());

        // for configure in &self.transition_configure.inner {
        //     manager
        //         .add_action(
        //             (self.id().into(), configure.transition()),
        //             configure.action(),
        //         )
        //         .await;
        // }

        let (sender, receiver) = tokio::sync::broadcast::channel::<EventMessage<E>>(100);

        self.status = true;
        self.sender = Some(sender);

        let state_machine = Arc::new(self);
        manager.start_up(state_machine.clone(), receiver);

        state_machine
    }

    pub async fn send_event(&self, message: EventMessage<E>) {
        if !self.status {
            return;
        }
        if let Some(sender) = self.sender.as_ref() {
            if let Err(e) = sender.send(message) {
                error!(
                    "DefaultStateMachine [{}] send event error: {}",
                    self.id(),
                    e
                );
                sender.closed().await
            }
        }
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

    pub fn add_state_listener<L>(mut self, listener: L) -> Self
    where
        L: DefaultStateMachineListener<S, E> + 'static,
    {
        self.listener = Some(Box::new(listener));
        self
    }

    pub fn remove_state_listener(&mut self) {
        self.listener = None;
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
            listener: Default::default(),
            sender: Default::default(),
            status: Default::default(),
        }
    }
}

#[async_trait]
pub trait DefaultStateMachineListener<S, E>: DynClone + Send + Sync {
    async fn state_changed(&self, from: S, to: S, event: E);

    async fn event_not_accepted(&self, event: EventMessage<E>);
}

next_web_core::clone_trait_object!(<S, E> DefaultStateMachineListener<S, E> where S: Clone , E: Clone,);

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct State<S, E> {
    pub source: S,
    pub event: E,
}

impl<S, E> Into<State<S, E>> for (S, E) {
    fn into(self) -> State<S, E> {
        State {
            source: self.0,
            event: self.1,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Transition<S, E> {
    pub source: S,
    pub target: S,
    pub event: E,
}

#[derive(Debug, Clone)]
pub struct EventMessage<E> {
    pub(crate) event: E,
    pub(crate) payload: Option<AnyValue>,
}

impl<E> EventMessage<E> {
    pub fn new(event: E) -> Self {
        Self {
            event,
            payload: None,
        }
    }

    pub fn payload(&self) -> Option<&AnyValue> {
        self.payload.as_ref()
    }

    pub fn event(&self) -> &E {
        &self.event
    }

    pub fn with_payload(mut self, payload: AnyValue) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn with_event(mut self, event: E) -> Self {
        self.event = event;
        self
    }
}

impl<E> Default for EventMessage<E>
where
    E: Default,
{
    fn default() -> Self {
        Self {
            event: Default::default(),
            payload: None,
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
            listener: self.listener.clone(),
            sender: self.sender.clone(),
            status: self.status.clone(),
        }
    }
}
