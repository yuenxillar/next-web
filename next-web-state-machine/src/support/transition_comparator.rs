use std::{cmp::Ordering, marker::PhantomData};

use crate::{
    support::state_machine_utils::StateMachineUtils,
    transition::{StateMachineTransition, transition_conflict_policy::TransitionConflictPolicy},
};

#[derive(Debug, Clone)]
pub struct TransitionComparator<S, E> {
    transition_conflict_policy: TransitionConflictPolicy,
    _marker: PhantomData<(S, E)>,
}

impl<S, E> TransitionComparator<S, E> {
    pub fn new(transition_conflict_policy: TransitionConflictPolicy) -> Self {
        TransitionComparator {
            transition_conflict_policy,

            _marker: PhantomData,
        }
    }

    /// 比较两个转换
    pub fn compare(
        &self,
        left: &dyn StateMachineTransition<S, E>,
        right: &dyn StateMachineTransition<S, E>,
    ) -> Ordering
    where
        S: PartialEq,
        S: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        #[cfg(feature = "trace-log")]
        tracing::trace!("Compare left='{:?}' right='{:?}'", left, right);

        if std::ptr::eq(left, right) {
            return Ordering::Equal;
        }

        let is_substate = StateMachineUtils::is_substate(left.source(), right.source());

        match self.transition_conflict_policy {
            TransitionConflictPolicy::Child => {
                if is_substate {
                    Ordering::Greater // 1 表示 left > right
                } else {
                    Ordering::Less // -1 表示 left < right
                }
            }
            TransitionConflictPolicy::Parent => {
                if is_substate {
                    Ordering::Less // -1 表示 left < right
                } else {
                    Ordering::Greater // 1 表示 left > right
                }
            }
        }
    }
}

impl<S, E> ToString for TransitionComparator<S, E> {
    fn to_string(&self) -> String {
        format!(
            "TransitionComparator [transitionConflictPolicy={:?}]",
            self.transition_conflict_policy
        )
    }
}

// ============ 可选：实现 Ord 和 PartialOrd 的包装器 ============

/// 可排序的转换包装器
pub struct SortableTransition<'a, S, E> {
    transition: &'a dyn StateMachineTransition<S, E>,
    comparator: &'a TransitionComparator<S, E>,
}

impl<'a, S, E> SortableTransition<'a, S, E> {
    pub fn new(
        transition: &'a dyn StateMachineTransition<S, E>,
        comparator: &'a TransitionComparator<S, E>,
    ) -> Self {
        SortableTransition {
            transition,
            comparator,
        }
    }
}

impl<'a, S, E> PartialEq for SortableTransition<'a, S, E>
where
    S: PartialEq,
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.comparator.compare(self.transition, other.transition) == Ordering::Equal
    }
}

impl<'a, S, E> Eq for SortableTransition<'a, S, E>
where
    S: PartialEq,
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
}

impl<'a, S, E> PartialOrd for SortableTransition<'a, S, E>
where
    S: PartialEq,
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, S, E> Ord for SortableTransition<'a, S, E>
where
    S: PartialEq,
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.comparator.compare(self.transition, other.transition)
    }
}
