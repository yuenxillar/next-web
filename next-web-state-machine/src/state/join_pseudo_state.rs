//! The join pseudo state.

use std::sync::Arc;
use std::sync::Mutex;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;

use crate::BoxedStateGuard;
use crate::state::StateMachineState;
use crate::state::composite_pseudo_state_listener::CompositePseudoStateListener;
use crate::state::default_pseudo_state_context::DefaultPseudoStateContext;
use crate::state::pseudo_state::PseudoState;
use crate::state::pseudo_state_context::{PseudoAction, PseudoStateContext};
use crate::state::pseudo_state_kind::PseudoStateKind;
use crate::state::pseudo_state_listener::PseudoStateListener;
use crate::state_context::StateContext;
use crate::support::lock_util::lock;

/// Data class wrapping a join target state and its guard.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct JoinStateData<S, E> {
    state: Arc<dyn StateMachineState<S, E>>,
    guard: Option<BoxedStateGuard<S, E>>,
}

impl<S, E> JoinStateData<S, E> {
    /// Creates a new join target.
    ///
    /// # Arguments
    /// * `state` - the target state
    /// * `guard` - the guard which selects this target
    pub fn new(state: Arc<dyn StateMachineState<S, E>>, guard: Option<BoxedStateGuard<S, E>>) -> Self {
        Self { state, guard }
    }

    /// Gets the target state.
    pub fn get_state(&self) -> &Arc<dyn StateMachineState<S, E>> {
        &self.state
    }

    /// Gets the guard of this target.
    pub fn get_guard(&self) -> Option<&BoxedStateGuard<S, E>> {
        self.guard.as_ref()
    }
}

/// The tracking state of a join.
struct JoinTracker<S> {
    track: Vec<Vec<S>>,
    notified: bool,
}

impl<S: Clone> JoinTracker<S> {
    fn new(joins: &[Vec<S>]) -> Self {
        Self {
            track: joins.to_vec(),
            notified: false,
        }
    }

    fn reset(&mut self, joins: &[Vec<S>]) {
        self.track = joins.to_vec();
        self.notified = false;
    }

    fn mark_completed(&mut self, state: &S) -> bool
    where
        S: PartialEq,
    {
        if self.notified {
            return true;
        }

        self.track.retain(|group| !group.iter().any(|item| item == state));

        if self.track.is_empty() {
            self.notified = true;
        }

        self.notified
    }
}

/// Join implementation of a [`PseudoState`].
///
/// The join completes when every state of every join group has completed. Only
/// then does entry continue to the first join target whose guard evaluates to
/// `true`.
///
/// # Type Parameters
/// * `S` - the type of state
/// * `E` - the type of event
pub struct JoinPseudoState<S, E> {
    joins: Vec<Vec<Arc<dyn StateMachineState<S, E>>>>,
    join_targets: Vec<JoinStateData<S, E>>,
    tracker: Mutex<JoinTracker<S>>,
    listeners: CompositePseudoStateListener<S, E>,
}

impl<S, E> JoinPseudoState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    /// Creates a new join pseudo state.
    ///
    /// # Arguments
    /// * `joins` - the groups of states that have to complete
    /// * `join_targets` - the candidate targets after the join
    pub fn new(
        joins: Vec<Vec<Arc<dyn StateMachineState<S, E>>>>,
        join_targets: Vec<JoinStateData<S, E>>,
    ) -> Self {
        let groups: Vec<Vec<S>> = joins
            .iter()
            .map(|group| group.iter().map(|state| state.id().clone()).collect())
            .collect();

        Self {
            joins,
            join_targets,
            tracker: Mutex::new(JoinTracker::new(&groups)),
            listeners: CompositePseudoStateListener::new(),
        }
    }

    /// Gets the join groups.
    pub fn get_joins(&self) -> &[Vec<Arc<dyn StateMachineState<S, E>>>] {
        &self.joins
    }

    /// Gets the join targets.
    pub fn get_join_targets(&self) -> &[JoinStateData<S, E>] {
        &self.join_targets
    }

    /// Checks whether the join has completed.
    pub fn is_notified(&self) -> bool {
        lock(&self.tracker).notified
    }

    /// Marks a state as completed.
    ///
    /// # Arguments
    /// * `state` - the id of the completed state
    pub fn mark_state_completed(&self, state: &S) {
        let completed = {
            let mut tracker = lock(&self.tracker);
            tracker.mark_completed(state)
        };

        if completed {
            self.notify_context(&DefaultPseudoStateContext::from_action(
                PseudoAction::JoinCompleted,
            ));
        }
    }

    /// Resets the join tracking to all groups.
    pub fn reset(&self) {
        let groups: Vec<Vec<S>> = self.joins.iter().map(|group| {
            group.iter().map(|state| state.id().clone()).collect()
        }).collect();
        lock(&self.tracker).reset(&groups);
    }

    /// Resets the join tracking and marks the given states as already joined.
    ///
    /// # Arguments
    /// * `ids` - the ids of the states which already joined
    pub fn reset_with_ids(&self, ids: &[S]) {
        self.reset();
        let mut tracker = lock(&self.tracker);
        tracker
            .track
            .retain(|group| !group.iter().any(|item| ids.contains(item)));
    }
}

impl<S, E> PseudoState<S, E> for JoinPseudoState<S, E>
where
    S: Clone + PartialEq + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn get_kind(&self) -> PseudoStateKind {
        PseudoStateKind::Join
    }

    fn entry<'a>(
        &'a self,
        context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Option<Arc<dyn StateMachineState<S, E>>>> {
        Box::pin(async move {
            if !self.is_notified() {
                return None;
            }

            for target in &self.join_targets {
                match target.guard.as_ref() {
                    Some(guard) => {
                        if guard(context).await {
                            return Some(target.state.clone());
                        }
                    }
                    None => return Some(target.state.clone()),
                }
            }

            None
        })
    }

    fn exit<'a>(
        &'a self,
        _context: &'a dyn StateContext<S, E>,
    ) -> BoxFuture<'a, Result<(), BoxError>> {
        Box::pin(async move {
            self.reset();
            Ok(())
        })
    }

    fn add_pseudo_state_listener(&self, listener: Arc<dyn PseudoStateListener<S, E>>) {
        self.listeners.register(listener);
    }

    fn set_pseudo_state_listeners(&self, listeners: Vec<Arc<dyn PseudoStateListener<S, E>>>) {
        self.listeners.set_listeners(listeners);
    }

    fn notify_context(&self, context: &dyn PseudoStateContext<S, E>) {
        self.listeners.on_context(context);
    }
}
