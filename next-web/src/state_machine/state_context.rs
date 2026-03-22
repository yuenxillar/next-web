use std::any::Any;

use next_web_core::{
    anys::any_value::AnyValue, error::BoxError, messaging::message_headers::MessageHeaders,
    traits::message::Message,
};

use crate::state_machine::{
    extended_state::ExtendedState, state::StateMachineState, transition::StateMachineTransition,
    StateMachine,
};

/// `StateContext` represents the current context used in various stages
/// of a state machine execution. This includes transitions, actions, and
/// guards in order to get access to event headers and extended state.
///
/// The context is not the current state of a state machine but more like
/// a snapshot of where the state machine is when this context is passed
/// to various methods.
pub trait StateContext<S, E>
where
    Self: Any,
    Self: Send + Sync,
{
    /// Gets the stage this context is attached to.
    fn stage(&self) -> Stage;

    /// Gets the message associated with this context.
    /// Message may be `None` if the transition is not triggered by a signal.
    fn message(&self) -> Option<&dyn Message<E>>;

    /// Gets the event associated with this context.
    /// Event may be `None` if the transition is not triggered by a signal.
    fn event(&self) -> Option<&E>;

    /// Gets the event message headers.
    fn message_headers(&self) -> Option<&MessageHeaders>;

    /// Gets a specific message header value.
    /// If the header key is not a `String`, the object's `to_string()` method
    /// is used to resolve the key name.
    fn message_header(&self, key: &str) -> Option<&AnyValue>;

    /// Gets the state machine's extended state.
    fn extended_state(&self) -> Option<&dyn ExtendedState>;

    /// Gets the current transition.
    fn transition(&self) -> Option<&dyn StateMachineTransition<S, E>>;

    /// Gets the state machine instance.
    fn state_machine(&self) -> Option<&dyn StateMachine<S, E>>;

    /// Gets the source state of this context.
    /// Generally, the source is where the state machine is coming from,
    /// which may be different than what the transition source is.
    fn source(&self) -> Option<&dyn StateMachineState<S, E>>;

    /// Gets the source states of this context.
    /// Multiple sources are only valid during a context when the machine
    /// is joining from multiple orthogonal regions.
    ///
    /// # See Also
    /// [`StateContext::source`]
    fn sources(&self) -> Option<Vec<&dyn StateMachineState<S, E>>>;

    /// Gets the target state of this context.
    /// Generally, the target is where the state machine is going to,
    /// which may be different than what the transition target is.
    fn target(&self) -> Option<&dyn StateMachineState<S, E>>;

    /// Gets the target states of this context.
    /// Multiple targets are only valid during a context when the machine
    /// is forking into multiple orthogonal regions.
    fn targets(&self) -> Option<Vec<&dyn StateMachineState<S, E>>>;

    /// Gets the error associated with this context.
    fn error(&self) -> Option<&BoxError>;

    fn set_error(&mut self, error: BoxError);
}

/// Represents a stage in the state machine execution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    /// Event was not accepted by the state machine
    EventNotAccepted,
    /// Extended state has changed
    ExtendedStateChanged,
    /// Main state has changed
    StateChanged,
    /// Entering a state
    StateEntry,
    /// Exiting a state
    StateExit,
    /// State machine encountered an error
    StateMachineError,
    /// State machine is starting
    StateMachineStart,
    /// State machine is stopping
    StateMachineStop,
    /// Transition is occurring
    Transition,
    /// Transition is starting
    TransitionStart,
    /// Transition is ending
    TransitionEnd,
}
