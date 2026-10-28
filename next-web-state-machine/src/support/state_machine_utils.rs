//! Utilities shared by the runtime.

use std::any::Any;
use std::sync::Arc;

use crate::state::StateMachineState;
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state_context::StateContext;

/// The message header which carries the timeout of a state do action.
pub const HEADER_DO_ACTION_TIMEOUT: &str = "STATEMACHINE_DO_ACTION_TIMEOUT";

/// State machine utility functions.
pub struct StateMachineUtils;

impl StateMachineUtils {
    /// Checks whether `right` is a substate of `left`.
    ///
    /// A state is a substate of another state when it lives in one of its
    /// regions or in its submachine.
    ///
    /// # Arguments
    /// * `left` - the potential parent state
    /// * `right` - the potential child state
    pub fn is_substate<S, E>(
        left: &Arc<dyn StateMachineState<S, E>>,
        right: &Arc<dyn StateMachineState<S, E>>,
    ) -> bool
    where
        S: PartialEq + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        if left.id() == right.id() {
            return false;
        }

        Self::state_contains(left, right.id())
    }

    /// Checks whether the given state contains a state with the given id.
    ///
    /// # Arguments
    /// * `state` - the state to inspect
    /// * `id` - the identifier to look for
    pub fn state_contains<S, E>(state: &Arc<dyn StateMachineState<S, E>>, id: &S) -> bool
    where
        S: PartialEq + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        for region in state.regions() {
            if let Some(current) = region.state()
                && current.id() == id {
                    return true;
                }
        }

        if let Some(submachine) = state.submachine() {
            for nested in submachine.states() {
                if nested.id() == id {
                    return true;
                }
            }
        }

        false
    }

    /// Checks whether at least one element of `left` is contained in `right`.
    ///
    /// # Arguments
    /// * `left` - the left collection
    /// * `right` - the right collection
    pub fn contains_at_least_one<T: PartialEq>(left: &[T], right: &[T]) -> bool {
        if left.is_empty() || right.is_empty() {
            return false;
        }
        left.iter().any(|item| right.contains(item))
    }

    /// Checks whether a state is a pseudo state which is not initial or end.
    ///
    /// # Arguments
    /// * `state` - the state to inspect
    pub fn is_normal_pseudo_state<S, E>(state: &dyn StateMachineState<S, E>) -> bool
    where
        S: PartialEq + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let Some(pseudo_state) = state.pseudo_state() else {
            return false;
        };

        !matches!(
            pseudo_state.get_kind(),
            PseudoStateKind::Initial | PseudoStateKind::End
        )
    }

    /// Checks whether a state is a transient pseudo state.
    ///
    /// # Arguments
    /// * `state` - the state to inspect
    pub fn is_transient_pseudo_state<S, E>(state: &dyn StateMachineState<S, E>) -> bool
    where
        S: PartialEq + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let Some(pseudo_state) = state.pseudo_state() else {
            return false;
        };

        matches!(
            pseudo_state.get_kind(),
            PseudoStateKind::Choice
                | PseudoStateKind::Junction
                | PseudoStateKind::Entry
                | PseudoStateKind::Exit
                | PseudoStateKind::HistoryDeep
                | PseudoStateKind::HistoryShallow
                | PseudoStateKind::Fork
                | PseudoStateKind::Join
        )
    }

    /// Checks whether a state carries a pseudo state of the given kind.
    ///
    /// # Arguments
    /// * `state` - the state to inspect
    /// * `kind` - the kind to compare against
    pub fn is_pseudo_state_of_kind<S, E>(
        state: &dyn StateMachineState<S, E>,
        kind: PseudoStateKind,
    ) -> bool
    where
        S: PartialEq + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        state
            .pseudo_state()
            .map(|pseudo_state| pseudo_state.get_kind() == kind)
            .unwrap_or(false)
    }

    /// Converts a collection into a collection of strings.
    ///
    /// # Arguments
    /// * `collection` - the collection to convert
    pub fn to_string_collection<T: ToString>(collection: &[T]) -> Vec<String> {
        collection.iter().map(ToString::to_string).collect()
    }

    /// Converts any value into a collection of strings.
    ///
    /// # Arguments
    /// * `object` - the value to convert
    pub fn to_string_collection_from_any(object: &dyn Any) -> Vec<String> {
        let mut result = Vec::new();

        if let Some(array) = object.downcast_ref::<Vec<&dyn Any>>() {
            for item in array {
                result.push(format!("{item:?}"));
            }
        } else {
            result.push(format!("{object:?}"));
        }

        result
    }

    /// Checks whether a collection of strings contains the given string.
    ///
    /// # Arguments
    /// * `left` - the collection to search
    /// * `right` - the value to look for
    pub fn contains_at_least_one_equal_string(left: &[String], right: &str) -> bool {
        left.iter().any(|item| item == right)
    }

    /// Checks whether two collections of strings share at least one element.
    ///
    /// # Arguments
    /// * `left` - the left collection
    /// * `right` - the right collection
    pub fn contains_at_least_one_equal_string_collection(
        left: &[String],
        right: &[String],
    ) -> bool {
        if left.is_empty() || right.is_empty() {
            return false;
        }

        left.iter().any(|item| right.contains(item))
    }

    /// Reads the do action timeout from the message headers.
    ///
    /// # Arguments
    /// * `context` - the state context
    pub fn get_message_header_do_action_timeout<S, E>(
        context: &dyn StateContext<S, E>,
    ) -> Option<i64>
    where
        S: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        context
            .message_headers()
            .and_then(|headers| headers.get(HEADER_DO_ACTION_TIMEOUT))
            .and_then(|value| value.as_i64())
    }
}
