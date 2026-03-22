use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;
use next_web_core::error::BoxError;
use next_web_core::messaging::message_headers::MessageHeaders;
use next_web_core::traits::message::Message;

use crate::state_machine::extended_state::ExtendedState;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::{Stage, StateContext};
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::StateMachine;

/// Default implementation of StateContext
pub struct DefaultStateContext<S, E> {
    stage: Stage,
    message: Option<Box<dyn Message<E>>>,
    message_headers: Option<MessageHeaders>,
    extended_state: Option<Arc<dyn ExtendedState>>,
    transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    state_machine: Option<Arc<dyn StateMachine<S, E>>>,
    source: Option<Arc<dyn StateMachineState<S, E>>>,
    target: Option<Arc<dyn StateMachineState<S, E>>>,
    sources: Option<Vec<Arc<dyn StateMachineState<S, E>>>>,
    targets: Option<Vec<Arc<dyn StateMachineState<S, E>>>>,
    error: Option<BoxError>,
}

impl<S, E> DefaultStateContext<S, E> {
    /// Creates a new DefaultStateContext with single source/target
    pub fn new(
        stage: Stage,
        message: Option<Box<dyn Message<E>>>,
        message_headers: Option<MessageHeaders>,
        extended_state: Option<Arc<dyn ExtendedState>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Option<Arc<dyn StateMachine<S, E>>>,
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Option<Arc<dyn StateMachineState<S, E>>>,
        error: Option<BoxError>,
    ) -> Self {
        Self {
            stage,
            message,
            message_headers,
            extended_state,
            transition,
            state_machine,
            source,
            target,
            sources: None,
            targets: None,
            error,
        }
    }

    /// Creates a new DefaultStateContext with multiple sources/targets
    pub fn new_with_collections(
        stage: Stage,
        message: Option<Box<dyn Message<E>>>,
        message_headers: Option<MessageHeaders>,
        extended_state: Option<Arc<dyn ExtendedState>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Option<Arc<dyn StateMachine<S, E>>>,
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Option<Arc<dyn StateMachineState<S, E>>>,
        sources: Option<Vec<Arc<dyn StateMachineState<S, E>>>>,
        targets: Option<Vec<Arc<dyn StateMachineState<S, E>>>>,
        error: Option<BoxError>,
    ) -> Self {
        Self {
            stage,
            message,
            message_headers,
            extended_state,
            transition,
            state_machine,
            source,
            target,
            sources,
            targets,
            error,
        }
    }
}

impl<S, E> StateContext<S, E> for DefaultStateContext<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    fn stage(&self) -> Stage {
        self.stage
    }

    fn event(&self) -> Option<&E> {
        self.message
            .as_ref()
            .map(|msg| msg.get_payload())
            .unwrap_or_default()
    }

    fn message(&self) -> Option<&dyn Message<E>> {
        self.message.as_deref()
    }

    fn message_headers(&self) -> Option<&MessageHeaders> {
        self.message_headers.as_ref()
    }

    fn message_header(&self, header: &str) -> Option<&AnyValue> {
        self.message_headers
            .as_ref()
            .map(|msg_headers| msg_headers.get(header))
            .unwrap_or_default()
    }

    fn extended_state(&self) -> Option<&dyn ExtendedState> {
        self.extended_state.as_deref()
    }

    fn transition(&self) -> Option<&dyn StateMachineTransition<S, E>> {
        self.transition.as_deref()
    }

    fn state_machine(&self) -> Option<&dyn StateMachine<S, E>> {
        self.state_machine.as_deref()
    }

    fn source(&self) -> Option<&dyn StateMachineState<S, E>> {
        match self.source.as_ref() {
            Some(source) => Some(source.as_ref()),
            None => self
                .transition
                .as_ref()
                .map(|transition| transition.source()),
        }
    }

    fn sources(&self) -> Option<Vec<&dyn StateMachineState<S, E>>> {
        self.sources
            .as_ref()
            .map(|sources| sources.iter().map(AsRef::as_ref).collect::<Vec<_>>())
    }

    fn target(&self) -> Option<&dyn StateMachineState<S, E>> {
        match self.target.as_ref() {
            Some(target) => Some(target.as_ref()),
            None => self
                .transition
                .as_ref()
                .map(|transition| transition.target()),
        }
    }

    fn targets(&self) -> Option<Vec<&dyn StateMachineState<S, E>>> {
        self.targets
            .as_ref()
            .map(|targets| targets.iter().map(AsRef::as_ref).collect::<Vec<_>>())
    }

    fn error(&self) -> Option<&BoxError> {
        self.error.as_ref()
    }

    fn set_error(&mut self, error: BoxError) {
        self.error = Some(error);
    }
}

impl<S, E> Debug for DefaultStateContext<S, E>
where
    S: Debug,
    S: Send + Sync,
    S: 'static,
    E: Debug,
    E: Send + Sync,
    E: 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("DefaultStateContext")
            .field("stage", &self.stage)
            .field("message", &self.message.as_ref().map(|m| m.get_payload()))
            .field("message_headers", &self.message_headers)
            .field(
                "extended_state",
                &self.extended_state.as_ref().map(|_| "ExtendedState"),
            )
            .field(
                "transition",
                &self.transition.as_ref().map(|_| "Transition"),
            )
            .field(
                "state_machine",
                &self.state_machine.as_ref().map(|_| "StateMachine"),
            )
            .field("source", &self.source.as_ref().map(|s| s.id()))
            .field("target", &self.target.as_ref().map(|t| t.id()))
            .field("sources", &self.sources.as_ref().map(|s| s.len()))
            .field("targets", &self.targets.as_ref().map(|t| t.len()))
            .field("error", &self.error)
            .finish()
    }
}
