use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};

use next_web_core::error::BoxError;

use crate::config::{
    action::{StateMachineAction, actions::Actions},
    builders::state_machine_transition_configurer::StateMachineTransitionConfigurer,
    common::builder::Builder,
    configurer_builder::ConfigurerBuilder,
    configurers::{
        choice_transition_configurer::ChoiceTransitionConfigurer,
        entry_transition_configurer::EntryTransitionConfigurer,
        exit_transition_configurer::ExitTransitionConfigurer,
        external_transition_configurer::ExternalTransitionConfigurer,
        fork_transition_configurer::ForkTransitionConfigurer,
        history_transition_configurer::HistoryTransitionConfigurer,
        internal_transition_configurer::InternalTransitionConfigurer,
        join_transition_configurer::JoinTransitionConfigurer,
        junction_transition_configurer::JunctionTransitionConfigurer,
        local_transition_configurer::LocalTransitionConfigurer,
        transition_configurer::TransitionConfigurer,
    },
    guard::StateMachineGuard,
    model::{
        choice_data::ChoiceData,
        entry_data::EntryData,
        exit_data::ExitData,
        history_data::HistoryData,
        junction_data::JunctionData,
        transition_data::{Guard, TransitionData},
        transitions_data::TransitionsData,
    },
};
use crate::{security::security_rule::SecurityRule, transition::transition_kind::TransitionKind};

fn builder_error(message: impl Into<String>) -> BoxError {
    std::io::Error::other(message.into()).into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveKind {
    External,
    Internal,
    Local,
    Choice,
    Junction,
    Fork,
    Join,
    Entry,
    Exit,
    History,
}

#[derive(Clone)]
struct PendingTransition<S, E> {
    source: Option<S>,
    target: Option<S>,
    state: Option<S>,
    event: Option<E>,
    period: Option<u64>,
    count: Option<u32>,
    actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    guard: Option<Arc<dyn StateMachineGuard<S, E>>>,
    security_rule: Option<SecurityRule>,
    name: Option<String>,
}

impl<S, E> PendingTransition<S, E> {
    fn new() -> Self {
        Self {
            source: None,
            target: None,
            state: None,
            event: None,
            period: None,
            count: None,
            actions: Vec::new(),
            guard: None,
            security_rule: None,
            name: None,
        }
    }
}

#[derive(Clone)]
struct TransitionSpec<S, E> {
    source: S,
    target: S,
    state: Option<S>,
    event: Option<E>,
    period: Option<u64>,
    count: Option<u32>,
    actions: Vec<crate::config::model::transition_data::Action<S, E>>,
    guard: Option<Guard<S, E>>,
    kind: TransitionKind,
    security_rule: Option<SecurityRule>,
    name: String,
}

#[derive(Clone)]
struct PendingBranch<S, E> {
    target: S,
    guard: Arc<dyn StateMachineGuard<S, E>>,
    actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
}

#[derive(Clone)]
struct PendingChoice<S, E> {
    source: Option<S>,
    branches: Vec<PendingBranch<S, E>>,
}

impl<S, E> PendingChoice<S, E> {
    fn new() -> Self {
        Self {
            source: None,
            branches: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct PendingFork<S> {
    source: Option<S>,
    targets: Vec<S>,
}

impl<S> PendingFork<S> {
    fn new() -> Self {
        Self {
            source: None,
            targets: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct PendingJoin<S> {
    target: Option<S>,
    sources: Vec<S>,
}

impl<S> PendingJoin<S> {
    fn new() -> Self {
        Self {
            target: None,
            sources: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct PendingPair<S> {
    source: Option<S>,
    target: Option<S>,
}

impl<S> PendingPair<S> {
    fn new() -> Self {
        Self {
            source: None,
            target: None,
        }
    }
}

#[derive(Clone)]
struct SharedState<S, E> {
    transition_specs: Vec<TransitionSpec<S, E>>,
    choices: HashMap<S, Vec<ChoiceData<S, E>>>,
    junctions: HashMap<S, Vec<JunctionData<S, E>>>,
    forks: HashMap<S, Vec<S>>,
    joins: HashMap<S, Vec<S>>,
    entry_data: Vec<EntryData<S, E>>,
    exit_data: Vec<ExitData<S, E>>,
    history_data: Vec<HistoryData<S, E>>,
}

impl<S, E> Default for SharedState<S, E> {
    fn default() -> Self {
        Self {
            transition_specs: Vec::new(),
            choices: HashMap::new(),
            junctions: HashMap::new(),
            forks: HashMap::new(),
            joins: HashMap::new(),
            entry_data: Vec::new(),
            exit_data: Vec::new(),
            history_data: Vec::new(),
        }
    }
}

#[derive(Clone)]
struct AlwaysTrueGuard;

impl<S, E> StateMachineGuard<S, E> for AlwaysTrueGuard
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn evaluate(&self, _ctx: &dyn crate::state_context::StateContext<S, E>) {}
}

#[derive(Clone)]
pub struct StateMachineTransitionBuilder<S, E>
where
    S: Eq + Hash,
{
    shared: Arc<Mutex<SharedState<S, E>>>,
    active_kind: Option<ActiveKind>,
    pending_transition: Option<PendingTransition<S, E>>,
    pending_choice: Option<PendingChoice<S, E>>,
    pending_junction: Option<PendingChoice<S, E>>,
    pending_fork: Option<PendingFork<S>>,
    pending_join: Option<PendingJoin<S>>,
    pending_entry: Option<PendingPair<S>>,
    pending_exit: Option<PendingPair<S>>,
    pending_history: Option<PendingPair<S>>,
}

impl<S, E> StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    pub fn new(_allow_configurers_of_same_type: bool) -> Self {
        Self {
            shared: Arc::new(Mutex::new(SharedState::default())),
            active_kind: None,
            pending_transition: None,
            pending_choice: None,
            pending_junction: None,
            pending_fork: None,
            pending_join: None,
            pending_entry: None,
            pending_exit: None,
            pending_history: None,
        }
    }

    fn root_view(&self) -> Self {
        Self {
            shared: self.shared.clone(),
            active_kind: None,
            pending_transition: None,
            pending_choice: None,
            pending_junction: None,
            pending_fork: None,
            pending_join: None,
            pending_entry: None,
            pending_exit: None,
            pending_history: None,
        }
    }

    fn with_kind(&self, kind: ActiveKind) -> Self {
        let mut next = self.root_view();
        next.active_kind = Some(kind);
        match kind {
            ActiveKind::External | ActiveKind::Internal | ActiveKind::Local => {
                next.pending_transition = Some(PendingTransition::new())
            }
            ActiveKind::Choice => next.pending_choice = Some(PendingChoice::new()),
            ActiveKind::Junction => next.pending_junction = Some(PendingChoice::new()),
            ActiveKind::Fork => next.pending_fork = Some(PendingFork::new()),
            ActiveKind::Join => next.pending_join = Some(PendingJoin::new()),
            ActiveKind::Entry => next.pending_entry = Some(PendingPair::new()),
            ActiveKind::Exit => next.pending_exit = Some(PendingPair::new()),
            ActiveKind::History => next.pending_history = Some(PendingPair::new()),
        }
        next
    }

    fn transition_guard(guard: Arc<dyn StateMachineGuard<S, E>>) -> Guard<S, E> {
        Arc::new(move |ctx| {
            guard.evaluate(ctx);
            Box::pin(async { true })
        })
    }

    fn transition_action(
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> crate::config::model::transition_data::Action<S, E> {
        Actions::from(action)
    }

    fn pending_transition_mut(&mut self) -> &mut PendingTransition<S, E> {
        self.pending_transition
            .as_mut()
            .expect("transition configurer is not active")
    }

    fn pending_choice_mut(&mut self) -> &mut PendingChoice<S, E> {
        self.pending_choice
            .as_mut()
            .expect("choice configurer is not active")
    }

    fn pending_junction_mut(&mut self) -> &mut PendingChoice<S, E> {
        self.pending_junction
            .as_mut()
            .expect("junction configurer is not active")
    }

    fn pending_fork_mut(&mut self) -> &mut PendingFork<S> {
        self.pending_fork
            .as_mut()
            .expect("fork configurer is not active")
    }

    fn pending_join_mut(&mut self) -> &mut PendingJoin<S> {
        self.pending_join
            .as_mut()
            .expect("join configurer is not active")
    }

    fn pending_entry_mut(&mut self) -> &mut PendingPair<S> {
        self.pending_entry
            .as_mut()
            .expect("entry configurer is not active")
    }

    fn pending_exit_mut(&mut self) -> &mut PendingPair<S> {
        self.pending_exit
            .as_mut()
            .expect("exit configurer is not active")
    }

    fn pending_history_mut(&mut self) -> &mut PendingPair<S> {
        self.pending_history
            .as_mut()
            .expect("history configurer is not active")
    }

    fn add_branch(
        pending: &mut PendingChoice<S, E>,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        actions: Vec<Arc<dyn StateMachineAction<S, E>>>,
    ) {
        pending.branches.push(PendingBranch {
            target,
            guard,
            actions,
        });
    }

    pub fn add_transition(
        &mut self,
        source: S,
        target: S,
        state: Option<S>,
        event: Option<E>,
        period: Option<u64>,
        count: Option<u32>,
        actions: Vec<crate::config::model::transition_data::Action<S, E>>,
        guard: Option<Guard<S, E>>,
        kind: TransitionKind,
        security_rule: Option<SecurityRule>,
        name: impl Into<String>,
    ) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .transition_specs
            .push(TransitionSpec {
                source,
                target,
                state,
                event,
                period,
                count,
                actions,
                guard,
                kind,
                security_rule,
                name: name.into(),
            });
    }

    pub fn add_choice(&mut self, source: S, choices: Vec<ChoiceData<S, E>>) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .choices
            .insert(source, choices);
    }

    pub fn add_junction(&mut self, source: S, junctions: Vec<JunctionData<S, E>>) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .junctions
            .insert(source, junctions);
    }

    pub fn add_entry(&mut self, source: S, target: S) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .entry_data
            .push(EntryData::new(source, target));
    }

    pub fn add_exit(&mut self, source: S, target: S) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .exit_data
            .push(ExitData::new(source, target));
    }

    pub fn add_fork(&mut self, source: S, targets: Vec<S>) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .forks
            .insert(source, targets);
    }

    pub fn add_join(&mut self, target: S, sources: Vec<S>) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .joins
            .insert(target, sources);
    }

    pub fn add_default_history(&mut self, source: S, target: S) {
        self.shared
            .lock()
            .expect("transition builder state poisoned")
            .history_data
            .push(HistoryData::new(source, target));
    }

    fn finalize_transition(&mut self, kind: ActiveKind) -> Result<(), BoxError> {
        let pending = self
            .pending_transition
            .take()
            .ok_or_else(|| builder_error("missing pending transition"))?;
        let source = pending
            .source
            .ok_or_else(|| builder_error("transition source must be configured"))?;
        let target = match kind {
            ActiveKind::Internal => pending.target.unwrap_or_else(|| source.clone()),
            _ => pending
                .target
                .ok_or_else(|| builder_error("transition target must be configured"))?,
        };
        let kind = match kind {
            ActiveKind::External => TransitionKind::External,
            ActiveKind::Internal => TransitionKind::Internal,
            ActiveKind::Local => TransitionKind::Local,
            _ => unreachable!(),
        };
        self.add_transition(
            source,
            target,
            pending.state,
            pending.event,
            pending.period,
            pending.count,
            pending
                .actions
                .into_iter()
                .map(Self::transition_action)
                .collect(),
            pending.guard.map(Self::transition_guard),
            kind,
            pending.security_rule,
            pending.name.unwrap_or_default(),
        );
        Ok(())
    }

    fn finalize_choice_like(&mut self, junction: bool) -> Result<(), BoxError> {
        let pending = if junction {
            self.pending_junction
                .take()
                .ok_or_else(|| builder_error("missing pending junction"))?
        } else {
            self.pending_choice
                .take()
                .ok_or_else(|| builder_error("missing pending choice"))?
        };
        let source = pending
            .source
            .ok_or_else(|| builder_error("choice/junction source must be configured"))?;
        if pending.branches.is_empty() {
            return Err(builder_error(
                "choice/junction requires at least one branch",
            ));
        }
        if junction {
            self.add_junction(
                source.clone(),
                pending
                    .branches
                    .into_iter()
                    .map(|b| {
                        if b.actions.is_empty() {
                            JunctionData::new(source.clone(), b.target, b.guard)
                        } else {
                            JunctionData::with_actions(source.clone(), b.target, b.guard, b.actions)
                        }
                    })
                    .collect(),
            );
        } else {
            self.add_choice(
                source.clone(),
                pending
                    .branches
                    .into_iter()
                    .map(|b| {
                        if b.actions.is_empty() {
                            ChoiceData::new(source.clone(), b.target, b.guard)
                        } else {
                            ChoiceData::with_actions(source.clone(), b.target, b.guard, b.actions)
                        }
                    })
                    .collect(),
            );
        }
        Ok(())
    }

    fn finalize_pair(pending: Option<PendingPair<S>>, what: &str) -> Result<(S, S), BoxError> {
        let pending = pending.ok_or_else(|| builder_error(format!("missing pending {what}")))?;
        let source = pending
            .source
            .ok_or_else(|| builder_error(format!("{what} source must be configured")))?;
        let target = pending
            .target
            .ok_or_else(|| builder_error(format!("{what} target must be configured")))?;
        Ok((source, target))
    }

    fn finalize_pending(&mut self) -> Result<(), BoxError> {
        let Some(kind) = self.active_kind.take() else {
            return Ok(());
        };
        match kind {
            ActiveKind::External | ActiveKind::Internal | ActiveKind::Local => {
                self.finalize_transition(kind)
            }
            ActiveKind::Choice => self.finalize_choice_like(false),
            ActiveKind::Junction => self.finalize_choice_like(true),
            ActiveKind::Fork => {
                let pending = self
                    .pending_fork
                    .take()
                    .ok_or_else(|| builder_error("missing pending fork"))?;
                let source = pending
                    .source
                    .ok_or_else(|| builder_error("fork source must be configured"))?;
                if pending.targets.is_empty() {
                    return Err(builder_error("fork requires at least one target"));
                }
                self.add_fork(source, pending.targets);
                Ok(())
            }
            ActiveKind::Join => {
                let pending = self
                    .pending_join
                    .take()
                    .ok_or_else(|| builder_error("missing pending join"))?;
                let target = pending
                    .target
                    .ok_or_else(|| builder_error("join target must be configured"))?;
                if pending.sources.is_empty() {
                    return Err(builder_error("join requires at least one source"));
                }
                self.add_join(target, pending.sources);
                Ok(())
            }
            ActiveKind::Entry => {
                let (source, target) = Self::finalize_pair(self.pending_entry.take(), "entry")?;
                self.add_entry(source, target);
                Ok(())
            }
            ActiveKind::Exit => {
                let (source, target) = Self::finalize_pair(self.pending_exit.take(), "exit")?;
                self.add_exit(source, target);
                Ok(())
            }
            ActiveKind::History => {
                let (source, target) = Self::finalize_pair(self.pending_history.take(), "history")?;
                self.add_default_history(source, target);
                Ok(())
            }
        }
    }
}

impl<S, E> Builder<TransitionsData<S, E>> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Eq + Send + Sync + 'static,
{
    fn build(&mut self) -> Result<TransitionsData<S, E>, BoxError> {
        self.finalize_pending()?;
        let state = self
            .shared
            .lock()
            .expect("transition builder state poisoned");
        let transitions = state
            .transition_specs
            .iter()
            .cloned()
            .map(|spec| {
                TransitionData::with_all(
                    spec.source,
                    spec.target,
                    spec.state,
                    spec.event,
                    spec.period,
                    spec.count,
                    spec.actions,
                    spec.guard,
                    spec.kind,
                    spec.security_rule,
                    spec.name,
                )
            })
            .collect();
        Ok(TransitionsData::with_extended(
            transitions,
            (!state.choices.is_empty()).then(|| state.choices.clone()),
            (!state.junctions.is_empty()).then(|| state.junctions.clone()),
            (!state.forks.is_empty()).then(|| state.forks.clone()),
            (!state.joins.is_empty()).then(|| state.joins.clone()),
            (!state.entry_data.is_empty()).then(|| state.entry_data.clone()),
            (!state.exit_data.is_empty()).then(|| state.exit_data.clone()),
            (!state.history_data.is_empty()).then(|| state.history_data.clone()),
        ))
    }
}

impl<S, E> StateMachineTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn with_external(&mut self) -> Result<Box<dyn ExternalTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::External)))
    }

    fn with_internal(&mut self) -> Result<Box<dyn InternalTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Internal)))
    }

    fn with_local(&mut self) -> Result<Box<dyn LocalTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Local)))
    }

    fn with_choice(&mut self) -> Result<Box<dyn ChoiceTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Choice)))
    }

    fn with_junction(&mut self) -> Result<Box<dyn JunctionTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Junction)))
    }

    fn with_fork(&mut self) -> Result<Box<dyn ForkTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Fork)))
    }

    fn with_join(&mut self) -> Result<Box<dyn JoinTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Join)))
    }

    fn with_entry(&mut self) -> Result<Box<dyn EntryTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Entry)))
    }

    fn with_exit(&mut self) -> Result<Box<dyn ExitTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::Exit)))
    }

    fn with_history(&mut self) -> Result<Box<dyn HistoryTransitionConfigurer<S, E>>, BoxError> {
        self.finalize_pending()?;
        Ok(Box::new(self.with_kind(ActiveKind::History)))
    }
}

impl<S, E> ExternalTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn target(&mut self, target: S) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().target = Some(target);
        self
    }
}

impl<S, E> InternalTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
}

impl<S, E> LocalTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn target(&mut self, target: S) -> &mut dyn LocalTransitionConfigurer<S, E> {
        self.pending_transition_mut().target = Some(target);
        self
    }
}

impl<S, E> ChoiceTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        self.pending_choice_mut().source = Some(source);
        self
    }

    fn first(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_choice_mut(), target, guard, Vec::new());
        self
    }

    fn first_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_choice_mut(), target, guard, vec![action]);
        self
    }

    fn first_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_choice_mut(),
            target,
            guard,
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }

    fn then(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_choice_mut(), target, guard, Vec::new());
        self
    }

    fn then_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_choice_mut(), target, guard, vec![action]);
        self
    }

    fn then_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_choice_mut(),
            target,
            guard,
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }

    fn last(&mut self, target: S) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_choice_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            Vec::new(),
        );
        self
    }

    fn last_with_action(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_choice_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            vec![action],
        );
        self
    }

    fn last_with_error(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ChoiceTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_choice_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }
}

impl<S, E> JunctionTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        self.pending_junction_mut().source = Some(source);
        self
    }

    fn first(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_junction_mut(), target, guard, Vec::new());
        self
    }

    fn first_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_junction_mut(), target, guard, vec![action]);
        self
    }

    fn first_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_junction_mut(),
            target,
            guard,
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }

    fn then(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_junction_mut(), target, guard, Vec::new());
        self
    }

    fn then_with_action(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(self.pending_junction_mut(), target, guard, vec![action]);
        self
    }

    fn then_with_error(
        &mut self,
        target: S,
        guard: Arc<dyn StateMachineGuard<S, E>>,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_junction_mut(),
            target,
            guard,
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }

    fn last(&mut self, target: S) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_junction_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            Vec::new(),
        );
        self
    }

    fn last_with_action(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_junction_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            vec![action],
        );
        self
    }

    fn last_with_error(
        &mut self,
        target: S,
        action: Arc<dyn StateMachineAction<S, E>>,
        error: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn JunctionTransitionConfigurer<S, E> {
        Self::add_branch(
            self.pending_junction_mut(),
            target,
            Arc::new(AlwaysTrueGuard),
            vec![Actions::error_calling_action(action, error)],
        );
        self
    }
}

impl<S, E> ForkTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn ForkTransitionConfigurer<S, E> {
        self.pending_fork_mut().source = Some(source);
        self
    }

    fn target(&mut self, target: S) -> &mut dyn ForkTransitionConfigurer<S, E> {
        self.pending_fork_mut().targets.push(target);
        self
    }
}

impl<S, E> JoinTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn JoinTransitionConfigurer<S, E> {
        self.pending_join_mut().sources.push(source);
        self
    }

    fn sources(&mut self, sources: Vec<S>) -> &mut dyn JoinTransitionConfigurer<S, E> {
        self.pending_join_mut().sources.extend(sources);
        self
    }

    fn target(&mut self, target: S) -> &mut dyn JoinTransitionConfigurer<S, E> {
        self.pending_join_mut().target = Some(target);
        self
    }
}

impl<S, E> EntryTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn EntryTransitionConfigurer<S, E> {
        self.pending_entry_mut().source = Some(source);
        self
    }

    fn target(&mut self, target: S) -> &mut dyn EntryTransitionConfigurer<S, E> {
        self.pending_entry_mut().target = Some(target);
        self
    }
}

impl<S, E> ExitTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn ExitTransitionConfigurer<S, E> {
        self.pending_exit_mut().source = Some(source);
        self
    }

    fn target(&mut self, target: S) -> &mut dyn ExitTransitionConfigurer<S, E> {
        self.pending_exit_mut().target = Some(target);
        self
    }
}

impl<S, E> HistoryTransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn HistoryTransitionConfigurer<S, E> {
        self.pending_history_mut().source = Some(source);
        self
    }

    fn target(&mut self, target: S) -> &mut dyn HistoryTransitionConfigurer<S, E> {
        self.pending_history_mut().target = Some(target);
        self
    }
}

impl<S, E> TransitionConfigurer<S, E> for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn source(&mut self, source: S) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().source = Some(source);
        self
    }

    fn state(&mut self, state: S) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().state = Some(state);
        self
    }

    fn event(&mut self, event: E) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().event = Some(event);
        self
    }

    fn timer(&mut self, period: u64) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        let pending = self.pending_transition_mut();
        pending.period = Some(period);
        pending.count = None;
        self
    }

    fn timer_once(&mut self, period: u64) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        let pending = self.pending_transition_mut();
        pending.period = Some(period);
        pending.count = Some(1);
        self
    }

    fn action(
        &mut self,
        action: Arc<dyn StateMachineAction<S, E>>,
    ) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().actions.push(action);
        self
    }

    fn guard(
        &mut self,
        guard: Arc<dyn StateMachineGuard<S, E>>,
    ) -> &mut dyn ExternalTransitionConfigurer<S, E> {
        self.pending_transition_mut().guard = Some(guard);
        self
    }
}

impl<S, E> ConfigurerBuilder<Box<dyn StateMachineTransitionConfigurer<S, E>>>
    for StateMachineTransitionBuilder<S, E>
where
    S: Clone + Eq + Hash + Send + Sync + 'static,
    E: Clone + Send + Sync + 'static,
{
    fn and(&mut self) -> Box<dyn StateMachineTransitionConfigurer<S, E>> {
        let _ = self.finalize_pending();
        Box::new(self.root_view())
    }
}
