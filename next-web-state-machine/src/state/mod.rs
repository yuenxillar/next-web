//! The state model of a state machine.
//!
//! A state is a region of its own: it can hold entry, exit and state actions,
//! deferred events, a pseudo state, orthogonal regions and a submachine. The
//! [`StateMachineState`](crate::state::StateMachineState) trait is the runtime
//! view of a state, [`ObjectState`](crate::state::object_state::ObjectState) is
//! the default implementation and
//! [`AbstractState`](crate::state::base_state::AbstractState) holds the behaviour
//! shared by every state implementation.

/// Listener of the actions a state executes.
pub mod action_listener;
/// The behaviour shared by every state implementation.
pub mod base_state;
/// The choice pseudo state.
pub mod choice_pseudo_state;
/// The listener composite of a pseudo state.
pub mod composite_pseudo_state_listener;
/// The listener composite of a state.
pub mod composite_state_listener;
/// The default pass-through pseudo state.
pub mod default_pseudo_state;
/// The default pseudo state context.
pub mod default_pseudo_state_context;
/// The entry point pseudo state.
pub mod entry_pseudo_state;
/// The exit point pseudo state.
pub mod exit_pseudo_state;
/// The fork pseudo state.
pub mod fork_pseudo_state;
/// The shallow and deep history pseudo states.
pub mod history_pseudo_state;
/// The join pseudo state.
pub mod join_pseudo_state;
/// The junction pseudo state.
pub mod junction_pseudo_state;
/// The default state implementation.
pub mod object_state;
/// The pseudo state contract.
pub mod pseudo_state;
/// The context passed to a pseudo state listener.
pub mod pseudo_state_context;
/// The kinds of a pseudo state.
pub mod pseudo_state_kind;
/// The pseudo state listener contract.
pub mod pseudo_state_listener;
/// The state backed by a region.
pub mod region_state;
/// A holder which breaks the build order cycle of the states.
pub mod state_holder;
/// The state listener contract.
pub mod state_listener;
/// An adapter which implements every state callback as a no-op.
pub mod state_listener_adapter;

pub use base_state::AbstractState;
pub use object_state::ObjectState;

use std::any::Any;
use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError, messaging::Message};

use crate::BoxedStateAction;
use crate::region::Region;
use crate::state::action_listener::ActionListener;
use crate::state::pseudo_state::PseudoState;
use crate::state::state_listener::StateListener;
use crate::state_context::StateContext;
use crate::state_machine::StateMachine;
use crate::state_machine_event_result::StateMachineEventResult;

/// `StateMachineState` is a state of a state machine.
///
/// The Java original separates `State`, `AbstractState`, `AbstractSimpleState`,
/// `ObjectState`, `EnumState`, `RegionState` and `StateMachineState`. The Rust
/// port keeps a single trait and gives [`ObjectState`] the behaviour that those
/// classes share, because Rust has no implementation inheritance.
///
/// # Type Parameters
/// - `S`: the type of state
/// - `E`: the type of event
#[async_trait]
pub trait StateMachineState<S, E>: Send + Sync + Any
where
    S: PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Sends an event to this state, returning one result per region that
    /// processed it.
    ///
    /// # Arguments
    /// * `event` - the wrapped event to send
    async fn send_event(
        &self,
        event: Arc<dyn Message<E>>,
    ) -> Vec<Arc<dyn StateMachineEventResult<S, E>>>;

    /// Checks if the state wants to defer the given event.
    ///
    /// # Arguments
    /// * `event` - the wrapped event
    fn should_defer(&self, event: &dyn Message<E>) -> bool;

    /// Initiates the exit sequence for the state.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn exit(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// Initiates the entry sequence for the state.
    ///
    /// # Arguments
    /// * `context` - the state context
    async fn entry(&self, context: &dyn StateContext<S, E>) -> Result<(), BoxError>;

    /// Gets the state identifier.
    fn id(&self) -> &S;

    /// Gets all identifiers of this state.
    ///
    /// Usually the returned slice contains a single identifier, except for an
    /// orthogonal state where the identifiers of all regions are returned.
    fn ids(&self) -> &[S];

    /// Gets all states this state knows about, excluding itself.
    ///
    /// For a simple state this is empty, for a compound state it contains the
    /// states of its regions.
    fn substates(&self) -> Vec<Arc<dyn StateMachineState<S, E>>>;

    /// Checks whether this state is addressed by the given identifier.
    ///
    /// A compound state is addressed by the identifiers of its regions and of
    /// its submachine too, which is what lets a transition that starts inside a
    /// region win over a transition of the enclosing state.
    ///
    /// # Arguments
    /// * `id` - the identifier to look for
    fn contains_id(&self, id: &S) -> bool {
        self.ids().iter().any(|item| item == id)
    }

    /// Gets the pseudo state attached to this state, if any.
    fn pseudo_state(&self) -> Option<Arc<dyn PseudoState<S, E>>>;

    /// Attaches a pseudo state to this state.
    ///
    /// The model is built bottom up, so a pseudo state is attached after its
    /// target states exist; the Java implementation reaches the same effect
    /// through `StateHolder`.
    ///
    /// # Arguments
    /// * `pseudo_state` - the pseudo state to attach, `None` removes it
    fn set_pseudo_state(&self, pseudo_state: Option<Arc<dyn PseudoState<S, E>>>);

    /// Gets the events deferred by this state.
    fn deferred_events(&self) -> &[E];

    /// Gets the actions executed when the state is entered.
    fn entry_actions(&self) -> &[BoxedStateAction<S, E>];

    /// Gets the actions executed once, right after entry.
    fn state_actions(&self) -> &[BoxedStateAction<S, E>];

    /// Gets the actions executed when the state is exited.
    fn exit_actions(&self) -> &[BoxedStateAction<S, E>];

    /// Gets the regions of this state.
    fn regions(&self) -> &[Arc<dyn Region<S, E>>] {
        &[]
    }

    /// Gets the submachine of this state, if it refers to one.
    fn submachine(&self) -> Option<&Arc<dyn StateMachine<S, E>>> {
        None
    }

    /// Checks if the state is a composite state.
    ///
    /// A composite state contains at least one region.
    fn is_composite(&self) -> bool {
        !self.regions().is_empty()
    }

    /// Checks if the state is an orthogonal state.
    ///
    /// An orthogonal state contains two or more regions.
    fn is_orthogonal(&self) -> bool {
        self.regions().len() > 1
    }

    /// Checks if the state is a submachine state.
    fn is_submachine_state(&self) -> bool {
        self.submachine().is_some()
    }

    /// Checks if the state is a simple state.
    ///
    /// A simple state has no regions and does not refer to a submachine.
    fn is_simple(&self) -> bool {
        !self.is_submachine_state() && !self.is_composite()
    }

    /// Adds a state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_state_listener(&self, listener: Arc<dyn StateListener<S, E>>);

    /// Removes a state listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_state_listener(&self, listener: Arc<dyn StateListener<S, E>>);

    /// Adds an action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn add_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>);

    /// Removes an action listener.
    ///
    /// # Arguments
    /// * `listener` - the listener
    fn remove_action_listener(&self, listener: Arc<dyn ActionListener<S, E>>);
}
