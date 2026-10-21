use std::{cmp::Ordering, marker::PhantomData};

use crate::{
    support::state_machine_utils::StateMachineUtils,
    transition::{StateMachineTransition, transition_conflict_policy::TransitionConflictPolicy},
};

/// Compares transitions so that the machine picks the same one the Java
/// implementation would.
///
/// The policy decides whether a transition of a child state wins over a
/// transition of its parent state.
#[derive(Debug, Clone)]
pub struct TransitionComparator<S, E> {
    transition_conflict_policy: TransitionConflictPolicy,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> TransitionComparator<S, E> {
    /// Creates a comparator for the given conflict policy.
    ///
    /// # Arguments
    /// * `transition_conflict_policy` - the conflict policy
    pub fn new(transition_conflict_policy: TransitionConflictPolicy) -> Self {
        TransitionComparator {
            transition_conflict_policy,
            _marker: PhantomData,
        }
    }

    /// Compares two transitions.
    ///
    /// # Arguments
    /// * `left` - the left transition
    /// * `right` - the right transition
    pub fn compare(
        &self,
        left: &dyn StateMachineTransition<S, E>,
        right: &dyn StateMachineTransition<S, E>,
    ) -> Ordering
    where
        S: Clone + PartialEq + Send + Sync + 'static,
        E: Clone + Send + Sync + 'static,
    {
        if std::ptr::eq(left, right) {
            return Ordering::Equal;
        }

        let (Some(left_source), Some(right_source)) = (left.source(), right.source()) else {
            return Ordering::Equal;
        };

        let is_substate = StateMachineUtils::is_substate(&left_source, &right_source);

        match self.transition_conflict_policy {
            TransitionConflictPolicy::Child => {
                if is_substate {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
            TransitionConflictPolicy::Parent => {
                if is_substate {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }
        }
    }
}
