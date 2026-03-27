pub mod action_listener;
pub mod base_state;
pub mod composite_pseudo_state_listener;
pub mod default_pseudo_state;
pub mod pseudo_state;
pub mod pseudo_state_context;
pub mod pseudo_state_kind;
pub mod pseudo_state_listener;
pub mod region_state;
pub mod state_holder;
pub mod state_listener;

use std::{any::Any, collections::HashSet, sync::Arc};

use futures::future::BoxFuture;
use next_web_core::{async_trait, traits::message::Message};

use crate::{
    state::{
        action_listener::ActionListener, pseudo_state::PseudoState, state_listener::StateListener,
    },
    state_context::StateContext,
    state_machine_event_result::{DefaultStateMachineEventResult, StateMachineEventResult},
};

/// `State` is an interface representing possible state in a state machine.
///
/// This trait defines the core functionality of a state within a state machine,
/// including event handling, entry/exit sequences, state classification,
/// and lifecycle management.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait StateMachineState<S, E, R = DefaultStateMachineEventResult<S, E>>
where
    R: StateMachineEventResult<S, E>,
    Self: Any,
    Self: Send + Sync,
{
    /// Send an event `E` wrapped with a `Message` to the state and
    /// return a `StateMachineEventResult` for results.
    ///
    /// # Arguments
    /// * `event` - the wrapped event to send
    ///
    /// # Returns
    /// A stream of state machine event results
    async fn send_event(&self, event: Box<dyn Message<E>>) -> R;

    /// Checks if state wants to defer an event.
    ///
    /// # Arguments
    /// * `event` - the wrapped event
    ///
    /// # Returns
    /// `true` if event should be deferred
    fn should_defer(&self, event: &dyn Message<E>) -> bool;

    /// Initiate an exit sequence for the state.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that completes when the exit sequence is finished
    async fn exit(&self, context: &dyn StateContext<S, E>);

    /// Initiate an entry sequence for the state.
    ///
    /// # Arguments
    /// * `context` - the state context
    ///
    /// # Returns
    /// A future that completes when the entry sequence is finished
    async fn entry(&self, context: &dyn StateContext<S, E>);

    /// Gets the state identifier.
    ///
    /// # Returns
    /// The state identifier
    fn id(&self) -> &S;

    /// Gets the state identifiers. Usually returned collection contains only one
    /// identifier except in a case where state is an orthogonal state.
    ///
    /// # Returns
    /// A collection of state identifiers
    fn ids(&self) -> HashSet<&S>;

    /// Gets all possible states this state knows about including itself
    /// and substates.
    ///
    /// # Returns
    /// A collection of states including itself and nested states
    fn states(&self) -> Vec<&dyn StateMachineState<S, E>>;

    /// Gets a `PseudoState` attached to a `State`.
    /// `PseudoState` is not required and thus this method returns
    /// `None` if it's not set.
    ///
    /// # Returns
    /// The pseudostate if present, otherwise `None`
    fn pseudo_state(&self) -> Option<&Arc<dyn PseudoState<S, E>>>;

    /// Gets the deferred events for this state.
    ///
    /// # Returns
    /// A collection of deferred events
    fn deferred_events(&self) -> &[E];

    /// Gets `Action`s executed entering in this state.
    ///
    /// # Returns
    /// A collection of entry action functions
    fn entry_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>];

    /// Gets `Action`s executed once in this state.
    ///
    /// # Returns
    /// A collection of state action functions
    fn state_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>];

    /// Gets `Action`s executed exiting from this state.
    ///
    /// # Returns
    /// A collection of exit action functions
    fn exit_actions(&self) -> &[Box<dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>>];

    /// Checks if state is a simple state. A simple state does not have any
    /// regions and it does not refer to any submachine state machine.
    ///
    /// # Returns
    /// `true` if state is a simple state
    fn is_simple(&self) -> bool;

    /// Checks if state is a composite state. A composite state is a state that
    /// contains at least one region.
    ///
    /// # Returns
    /// `true` if state is a composite state
    fn is_composite(&self) -> bool;

    /// Checks if state is an orthogonal state. An orthogonal composite state
    /// contains two or more regions. If this method returns `true`,
    /// `is_composite()` will also always return `true`.
    ///
    /// # Returns
    /// `true` if state is an orthogonal state
    fn is_orthogonal(&self) -> bool;

    /// Checks if state is a submachine state. This kind of state refers to a
    /// state machine(submachine).
    ///
    /// # Returns
    /// `true` if state is a submachine state
    fn is_submachine_state(&self) -> bool;

    /// Adds the state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_state_listener(&mut self, listener: Arc<dyn StateListener<S, E>>);

    /// Removes the state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_state_listener(&mut self, listener: &dyn StateListener<S, E>);

    /// Adds the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_action_listener(&mut self, listener: Arc<dyn ActionListener<S, E>>);

    /// Removes the action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_action_listener(&mut self, listener: &dyn ActionListener<S, E>);
}
