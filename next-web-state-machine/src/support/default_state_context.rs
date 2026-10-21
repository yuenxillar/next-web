//! Default implementation of the state context.

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::Arc;

use next_web_core::any::AnyValue;
use next_web_core::error::BoxError;
use next_web_core::messaging::Message;
use next_web_core::messaging::message_headers::MessageHeaders;

use crate::StateMachine;
use crate::extended_state::ExtendedState;
use crate::state::StateMachineState;
use crate::state_context::{Stage, StateContext};
use crate::transition::StateMachineTransition;

/// Default implementation of [`StateContext`].
///
/// The message headers are read through the message instead of being copied,
/// so building a context per stage does not cost an allocation.
pub struct DefaultStateContext<S, E> {
    stage: Stage,
    message: Option<Arc<dyn Message<E>>>,
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
    /// Creates a new state context with a single source and target.
    ///
    /// # Arguments
    /// * `stage` - the stage of the context
    /// * `message` - the message behind the context
    /// * `message_headers` - headers which override the ones of the message
    /// * `extended_state` - the extended state
    /// * `transition` - the transition behind the context
    /// * `state_machine` - the state machine
    /// * `source` - the source state
    /// * `target` - the target state
    /// * `error` - the error, if the context reports one
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stage: Stage,
        message: Option<Arc<dyn Message<E>>>,
        message_headers: Option<MessageHeaders>,
        extended_state: Option<Arc<dyn ExtendedState>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Option<Arc<dyn StateMachine<S, E>>>,
        source: Option<Arc<dyn StateMachineState<S, E>>>,
        target: Option<Arc<dyn StateMachineState<S, E>>>,
        error: Option<BoxError>,
    ) -> Self {
        let _ = message_headers;
        // The Java context falls back to the ends of the transition; storing the
        // fallback here keeps `source()` and `target()` borrowable.
        let source = source.or_else(|| transition.as_ref().and_then(|item| item.source()));
        let target = target.or_else(|| transition.as_ref().map(|item| item.target()));
        Self {
            stage,
            message,
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

    /// Creates a new state context with several sources and targets.
    ///
    /// # Arguments
    /// * `stage` - the stage of the context
    /// * `message` - the message behind the context
    /// * `extended_state` - the extended state
    /// * `transition` - the transition behind the context
    /// * `state_machine` - the state machine
    /// * `sources` - the source states
    /// * `targets` - the target states
    #[allow(clippy::too_many_arguments)]
    pub fn with_collections(
        stage: Stage,
        message: Option<Arc<dyn Message<E>>>,
        extended_state: Option<Arc<dyn ExtendedState>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Option<Arc<dyn StateMachine<S, E>>>,
        sources: Vec<Arc<dyn StateMachineState<S, E>>>,
        targets: Vec<Arc<dyn StateMachineState<S, E>>>,
    ) -> Self {
        Self {
            stage,
            message,
            extended_state,
            transition,
            state_machine,
            source: None,
            target: None,
            sources: Some(sources),
            targets: Some(targets),
            error: None,
        }
    }
}

impl<S, E> StateContext<S, E> for DefaultStateContext<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn stage(&self) -> Stage {
        self.stage
    }

    fn event(&self) -> Option<&E> {
        self.message
            .as_ref()
            .and_then(|message| message.get_payload())
    }

    fn message(&self) -> Option<&dyn Message<E>> {
        self.message.as_deref()
    }

    fn message_arc(&self) -> Option<Arc<dyn Message<E>>> {
        self.message.clone()
    }

    fn message_headers(&self) -> Option<&MessageHeaders> {
        self.message
            .as_ref()
            .map(|message| message.get_headers())
    }

    fn message_header(&self, header: &str) -> Option<&AnyValue> {
        self.message_headers()
            .and_then(|headers| headers.get(header))
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
        self.source.as_deref()
    }

    fn sources(&self) -> Option<Vec<&dyn StateMachineState<S, E>>> {
        self.sources
            .as_ref()
            .map(|sources| sources.iter().map(AsRef::as_ref).collect())
    }

    fn target(&self) -> Option<&dyn StateMachineState<S, E>> {
        self.target.as_deref()
    }

    fn targets(&self) -> Option<Vec<&dyn StateMachineState<S, E>>> {
        self.targets
            .as_ref()
            .map(|targets| targets.iter().map(AsRef::as_ref).collect())
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
    S: Debug + PartialEq + Send + Sync + 'static,
    E: Debug + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("DefaultStateContext")
            .field("stage", &self.stage)
            .field("message", &self.message.as_ref().map(|m| m.get_payload()))
            .field("source", &self.source.as_ref().map(|s| s.id()))
            .field("target", &self.target.as_ref().map(|t| t.id()))
            .field("error", &self.error.as_ref().map(ToString::to_string))
            .finish()
    }
}
