use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::messaging::message_headers::MessageHeaders;
use next_web_core::traits::message::Message;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use uuid::Uuid;

use crate::state_machine::access::state_machine_accessor::StateMachineAccessor;
use crate::state_machine::extended_state::{ExtendedState, ExtendedStateChangeListener};
use crate::state_machine::listener::state_machine_listener::StateMachineListener;
use crate::state_machine::region::Region;
use crate::state_machine::state::base_state::BaseState;
use crate::state_machine::state::pseudo_state::PseudoState;
use crate::state_machine::state::pseudo_state_kind::PseudoStateKind;
use crate::state_machine::state::state_listener::StateListener;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::{Stage, StateContext};
use crate::state_machine::state_machine_context::StateMachineContext;
use crate::state_machine::state_machine_event_result::{
    DefaultStateMachineEventResult, StateMachineEventResult,
};
use crate::state_machine::support::default_extended_state::DefaultExtendedState;
use crate::state_machine::support::default_state_context::DefaultStateContext;
use crate::state_machine::support::default_state_machine_executor::DefaultStateMachineExecutor;
use crate::state_machine::support::lifecycle_object_support::LifecycleObjectSupportExt;
use crate::state_machine::support::state_machine_executor::StateMachineExecutor;
use crate::state_machine::support::state_machine_object_support::{
    StateMachineListenerRelay, StateMachineObjectSupport, StateMachineObjectSupportExt,
};
use crate::state_machine::transition::initial_transition::InitialTransition;
use crate::state_machine::transition::transition_conflict_policy::TransitionConflictPolicy;
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::StateMachine;

/// These will be imported by the user
pub struct BaseStateMachine<S, E> {
    states: Vec<Arc<dyn StateMachineState<S, E>>>,
    transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    initial_state: Arc<dyn StateMachineState<S, E>>,
    initial_transition: Arc<dyn StateMachineTransition<S, E>>,
    initial_event: Option<Box<dyn Message<E>>>,
    extended_state: Arc<dyn ExtendedState>,
    transition_conflict_policy: Option<TransitionConflictPolicy>,
    current_state: Option<Arc<dyn StateMachineState<S, E>>>,
    last_state: Option<Arc<dyn StateMachineState<S, E>>>,
    current_error: Option<BoxError>,
    history: Option<Arc<dyn PseudoState<S, E>>>,
    trigger_to_transition: HashMap<String, Arc<dyn StateMachineTransition<S, E>>>,
    triggerless_transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    relay: Option<Arc<dyn StateMachine<S, E>>>,
    state_machine_executor: Option<Arc<dyn StateMachineExecutor<S, E>>>,
    initial_enabled: Option<bool>,
    uuid: Uuid,
    id: Option<String>,
    forwarded_initial_event: Option<Box<dyn Message<E>>>,
    parent_machine: Option<Arc<dyn StateMachine<S, E>>>,

    object_support: StateMachineObjectSupport<S, E>,
}

impl<S, E> BaseStateMachine<S, E>
where
    S: Send + Sync,
    S: 'static,
    E: Send + Sync,
    E: 'static,
{
    pub fn new(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
    ) -> Self {
        Self::with_extended_state(
            states,
            transitions,
            initial_state,
            Arc::new(DefaultExtendedState::default()),
        )
    }

    pub fn with_extended_state(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        extended_state: Arc<dyn ExtendedState>,
    ) -> Self {
        Self::with_all(
            states,
            transitions,
            initial_state,
            None,
            None,
            extended_state,
            None,
        )
    }

    pub fn with_all(
        states: Vec<Arc<dyn StateMachineState<S, E>>>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_state: Arc<dyn StateMachineState<S, E>>,
        initial_transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        initial_event: Option<Box<dyn Message<E>>>,
        extended_state: Arc<dyn ExtendedState>,
        uuid: Option<Uuid>,
    ) -> Self {
        let uuid = uuid.unwrap_or_else(Uuid::new_v4);

        let initial_transition = initial_transition
            .unwrap_or_else(|| Arc::new(InitialTransition::new(initial_state.clone())));

        BaseStateMachine {
            states,
            transitions,
            initial_state,
            initial_transition,
            initial_event,
            extended_state,
            transition_conflict_policy: None,
            current_state: None,
            last_state: None,
            current_error: None,
            history: None,
            trigger_to_transition: HashMap::new(),
            triggerless_transitions: Vec::new(),
            relay: None,
            state_machine_executor: None,
            initial_enabled: None,
            uuid,
            id: None,
            forwarded_initial_event: None,
            parent_machine: None,
            object_support: Default::default(),
        }
    }

    pub fn set_history_state(&mut self, history: Arc<dyn PseudoState<S, E>>) {
        let _ = self.history.replace(history);
    }

    pub fn get_history_state(&self) -> Option<&dyn PseudoState<S, E>> {
        self.history.as_deref()
    }

    // pub async fn send_event(&self, event: Box<dyn Message<E>>) -> bool {
    //     // Simplified implementation
    //     if self.has_state_machine_error().await {
    //         return false;
    //     }

    //     let event = self.pre_event(event).await;
    //     self.accept_event(event).await
    // }

    // pub async fn send_events(
    //     &self,
    //     events: Vec<Box<dyn Message<E>>>,
    // ) -> Vec<StateMachineEventResult<S, E>> {
    //     let mut results = Vec::new();
    //     for event in events {
    //         results.push(self.handle_event(event).await);
    //     }
    //     results
    // }

    // async fn handle_event(&self, event: Box<dyn Message<E>>) -> StateMachineEventResult<S, E> {
    //     if self.has_state_machine_error().await {
    //         return StateMachineEventResult::denied(self, event);
    //     }

    //     let event = self.pre_event(event).await;
    //     self.accept_event(event).await;

    //     DefaultStateMachineEventResult::accepted(self, event)
    // }

    // async fn pre_event(&self, event: Box<dyn Message<E>>) -> Box<dyn Message<E>> {
    //     // Apply interceptors
    //     let mut event = event;
    //     for interceptor in self.state_machine_interceptors.iter() {
    //         event = interceptor.pre_event(event, self).await;
    //     }
    //     event
    // }

    // async fn accept_event(&self, event: Box<dyn Message<E>>) -> bool {
    //     let current_state_opt = self.current_state.clone();

    //     if let Some(current_state) = current_state_opt {
    //         if current_state.should_defer(&event).await {
    //             // Queue deferred event
    //             if let Some(executor) = self.state_machine_executor.as_ref() {
    //                 executor.queue_deferred_event(event).await;
    //             }
    //             return false;
    //         }

    //         // Send event to current state
    //         if current_state.send_event(event.clone()).await {
    //             return true;
    //         }

    //         // Check transitions
    //         for transition in &self.transitions {
    //             if let Some(trigger) = transition.trigger() {
    //                 if transition.source().ids().contains(&current_state.id()) {
    //                     if trigger.evaluate(event.payload().clone()).await {
    //                         if let Some(executor) = self.state_machine_executor.as_ref() {
    //                             executor.queue_event(event).await;
    //                             return true;
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     self.notify_event_not_accepted(event).await;
    //     false
    // }

    // async fn notify_event_not_accepted(&self, event: Box<dyn Message<E>>) {
    //     if self.parent_machine.is_none() {
    //         // Notify listeners
    //         for listener in self.object_support.iter() {
    //             listener.event_not_accepted(event.clone()).await;
    //         }
    //     }
    // }

    // async fn notify_state_machine_started(&self) {
    //     let context = self.build_state_context(
    //         Stage::StateMachineStart,
    //         None,
    //         None,
    //         self.get_relay_state_machine(),
    //     );

    //     for listener in self.state_listeners.iter() {
    //         listener.state_machine_started(context.clone()).await;
    //     }
    // }

    // async fn notify_state_machine_stopped(&self) {
    //     let context = self.build_state_context(Stage::StateMachineStop, None, None, self);

    //     for listener in self.state_listeners.iter() {
    //         listener.state_machine_stopped(context.clone()).await;
    //     }
    // }

    // async fn notify_state_machine_error(&self, error: Box<dyn std::error::Error + Send + Sync>) {
    //     let context =
    //         self.build_state_context_with_error(Stage::StateMachineError, None, None, self, error);

    //     for listener in self.state_listeners.iter() {
    //         listener.state_machine_error(context.clone()).await;
    //     }
    // }

    fn build_state_context(
        &self,
        stage: Stage,
        message: Option<Box<dyn Message<E>>>,
        transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
        state_machine: Arc<dyn StateMachine<S, E>>,
    ) -> DefaultStateContext<S, E> {
        let message_headers = match message.as_ref() {
            Some(msg) => msg.get_headers().clone(),
            None => MessageHeaders::default(),
        };

        DefaultStateContext::new(
            stage,
            message,
            Some(message_headers),
            Some(self.extended_state.clone()),
            transition,
            Some(state_machine),
            None,
            None,
            None,
        )
    }

    // fn build_state_context_with_error(
    //     &self,
    //     stage: Stage,
    //     message: Option<Box<dyn Message<E>>>,
    //     transition: Option<Arc<dyn StateMachineTransition<S, E>>>,
    //     state_machine: Arc<dyn StateMachine<S, E>>,
    //     error: Box<dyn std::error::Error + Send + Sync>,
    // ) -> Arc<dyn StateContext<S, E>> {
    //     DefaultStateContext::new_with_error(
    //         stage,
    //         message,
    //         self.extended_state.clone(),
    //         transition,
    //         state_machine,
    //         error,
    //     )
    // }

    fn get_relay_state_machine(&self) -> Arc<dyn StateMachine<S, E>> {
        self.relay
            .as_ref()
            .map(Clone::clone)
            .unwrap_or(Arc::new(self.clone()))
    }

    // pub async fn get_state(&self) -> Option<Arc<dyn StateMachineState<S, E>>> {
    //     if self.is_complete().await {
    //         self.last_state.clone()
    //     } else {
    //         self.current_state.clone()
    //     }
    // }

    // pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
    //     if self.current_state.is_some() {
    //         debug!("State already set, disabling initial");
    //         self.register_pseudo_state_listener().await;
    //         if let Some(executor) = self.state_machine_executor.as_ref() {
    //             executor.set_initial_enabled(false);
    //         }
    //     } else {
    //         self.do_start().await?;
    //         self.register_pseudo_state_listener().await;

    //         if let Some(executor) = self.state_machine_executor.as_ref() {
    //             if let Some(initial_enabled) = self.initial_enabled.as_ref() {
    //                 if !initial_enabled {
    //                     debug!("Initial disable asked, disabling initial");
    //                     executor.set_initial_enabled(false);
    //                 } else {
    //                     executor.set_forwarded_initial_event(self.forwarded_initial_event.clone());
    //                 }
    //             }
    //         }
    //     }

    //     if let Some(executor) = self.state_machine_executor.as_ref() {
    //         executor.start().await?;
    //     }

    //     self.notify_state_machine_started().await;
    //     Ok(())
    // }

    // async fn do_start(&self) -> Result<(), Box<dyn std::error::Error>> {
    //     // Base start logic
    //     Ok(())
    // }

    // pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
    //     self.notify_state_machine_stopped().await;

    //     // Stash current state before nulling it
    //     self.last_state = self.current_state.clone();
    //     self.current_state = None;
    //     self.initial_enabled = None;

    //     debug!("Stop complete");

    //     if let Some(executor) = self.state_machine_executor.as_ref() {
    //         executor.stop().await?;
    //     }

    //     Ok(())
    // }

    // pub async fn is_complete(&self) -> bool {
    //     if let Some(state) = self.current_state.as_ref() {
    //         if let Some(pseudo_state) = state.pseudo_state() {
    //             return pseudo_state.get_kind() == PseudoStateKind::End;
    //         }
    //     }
    //     !self.is_running().await
    // }

    // pub async fn is_running(&self) -> bool {
    //     // Simplified - would need actual running state tracking
    //     self.current_state.is_some()
    // }

    // async fn register_pseudo_state_listener(&self) {
    //     for state in &self.states {
    //         if let Some(pseudo_state) = state.pseudo_state() {
    //             let listeners = vec![Arc::new(PseudoStateListenerImpl::new(
    //                 self.clone(),
    //                 pseudo_state.clone(),
    //             ))];
    //             // Would need to set listeners on pseudo_state
    //         }
    //     }
    // }

    // pub fn add_state_listener(&self, listener: Arc<dyn StateMachineListener<S, E>>) {
    //     self.state_listeners.push(listener);
    // }

    // pub fn remove_state_listener(&self, listener: Arc<dyn StateMachineListener<S, E>>) {
    //     self.state_listeners
    //         .write()
    //         .unwrap()
    //         .retain(|l| !Arc::ptr_eq(l, &listener));
    // }

    // pub async fn set_state_machine_error(
    //     &self,
    //     exception: Option<Box<dyn std::error::Error + Send + Sync>>,
    // ) {
    //     let exception = if let Some(exception) = exception {
    //         // Apply interceptors
    //         let mut exception = exception;
    //         for interceptor in self.state_machine_interceptors.iter() {
    //             exception = interceptor.state_machine_error(self, exception).await;
    //         }
    //         Some(exception)
    //     } else {
    //         None
    //     };

    //     *self.current_error = exception;

    //     if let Some(error) = self.current_error.as_ref() {
    //         self.notify_state_machine_error(error.clone()).await;
    //     }
    // }

    // pub async fn has_state_machine_error(&self) -> bool {
    //     self.current_error.is_some()
    // }

    // pub fn get_states(&self) -> Vec<Arc<dyn StateMachineState<S, E>>> {
    //     self.states.clone()
    // }

    // pub fn get_transitions(&self) -> Vec<Arc<dyn StateMachineTransition<S, E>>> {
    //     self.transitions.clone()
    // }

    // pub fn set_initial_enabled(&mut self, enabled: bool) {
    //     self.initial_enabled = Some(enabled);
    // }

    pub fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn get_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn set_id(&mut self, id: impl Into<String>) {
        self.id = Some(id.into());
    }

    async fn execute_triggerless_transitions(
        &self,
        state_context: &dyn StateContext<S, E>,
        state: &dyn StateMachineState<S, E>,
    ) {
        if let Some(state_machine_executor) = self.state_machine_executor.as_ref() {
            state_machine_executor
                .execute_triggerless_transitions(state_context, state)
                .await;
            if let Some(current_state) = self.current_state.as_ref() {
                if current_state.is_orthogonal() {
                    let any: &dyn Any = current_state.as_ref();
                    any.downcast_ref::<ObjectState>()
                } else if current_state.is_submachine_state() {
                    current_state.state_
                }

                return;
            }
        }
    }

    // pub fn set_relay(&mut self, state_machine: Arc<dyn StateMachine<S, E>>) {
    //     self.relay = Some(state_machine);
    // }

    // pub fn set_parent_machine(&mut self, parent_machine: Arc<dyn StateMachine<S, E>>) {
    //     self.parent_machine = Some(parent_machine);
    // }

    // pub fn set_forwarded_initial_event(&mut self, message: Box<dyn Message<E>>) {
    //     self.forwarded_initial_event = Some(message);
    // }

    // pub async fn reset_state_machine(
    //     &self,
    //     state_machine_context: &dyn StateMachineContext<S, E>,
    // ) -> Result<(), Box<dyn std::error::Error>> {
    //     self.reset_state_machine_reactively(state_machine_context)
    //         .await
    // }

    // pub async fn reset_state_machine_reactively(
    //     &self,
    //     context: &dyn StateMachineContext<S, E>,
    // ) -> Result<(), Box<dyn std::error::Error>>
    // where
    //     S: Eq + Hash,
    // {
    //     // debug!("Request to reset state machine: {:?}", context);
    //     self.set_id(context.id());

    //     let state_id = context.state();
    //     for state in &self.states {
    //         for substate in state.states() {
    //             if substate.ids().contains(&state_id) {
    //                 self.current_state = Some(state.clone());
    //                 self.last_state = Some(state.clone());

    //                 // Reset extended state
    //                 let extended_state_context = context.extended_state();
    //                 let mut ext_state = self.extended_state;
    //                 ext_state.variables().clear();
    //                 for (k, v) in extended_state_context.variables() {
    //                     ext_state.variables().insert(k.clone(), v.clone());
    //                 }

    //                 debug!("State reset");
    //                 break;
    //             }
    //         }
    //     }

    //     if let Some(history_state) = self.history.as_ref() {
    //         let history_states = context.history_states();
    //         for (key, state_id) in history_states {
    //             for h in self.get_states() {
    //                 if h.id() == state_id {
    //                     // Would need to set history state
    //                     break;
    //                 }
    //             }
    //         }
    //     } else {
    //         info!("Got null context, resetting to initial state");
    //         self.current_state = Some(self.initial_state.clone());
    //         self.extended_state.variables().clear();
    //         self.set_id(String::new());
    //     }

    //     Ok(())
    // }

    // pub fn add_state_machine_interceptor(
    //     &mut self,
    //     interceptor: Arc<dyn StateMachineInterceptor<S, E>>,
    // ) {
    //     self.object_support
    //         .state_machine_interceptors
    //         .push(interceptor);
    // }

    // pub fn add_state_machine_monitor(&mut self, monitor: Arc<dyn StateMachineMonitor<S, E>>) {
    //     self.state_machine_monitors.push(monitor);
    // }
}

#[async_trait]
impl<S, E> Region<S, E> for BaseStateMachine<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn id(&self) -> &str {
        todo!()
    }

    /// Start the region.
    async fn start(&self) -> Result<(), BoxError> {
        todo!()
    }

    /// Stop the region.
    async fn stop(&self) -> Result<(), BoxError> {
        todo!()
    }

    /// Send an event to the region.
    async fn send_event(
        &self,
        event: Box<dyn Message<E>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
        todo!()
    }

    /// Send multiple events to the region.
    async fn send_events(
        &self,
        events: Vec<Box<dyn Message<E>>>,
    ) -> Result<Box<dyn StateMachineEventResult<S, E>>, BoxError> {
        todo!()
    }

    /// Gets the current State.
    fn state(&self) -> Option<&dyn StateMachineState<S, E>> {
        // if we're complete assume we're stopped
        // and state was stashed into lastState
        if let Some(state) = self.last_state.as_deref() {
            if self.is_complete() {
                return Some(state);
            }
        }

        self.current_state.as_deref()
    }

    /// Gets the current States.
    fn states(&self) -> Vec<&dyn StateMachineState<S, E>> {
        todo!()
    }

    /// Gets a Transitions for this region.
    fn transitions(&self) -> Vec<&dyn StateMachineTransition<S, E>> {
        todo!()
    }

    /// Checks if region complete.
    /// Region is considered to be completed if it has reached its end state and no further event processing is happening.
    fn is_complete(&self) -> bool {
        todo!()
    }

    /// Adds the state listener.
    fn add_state_listener(
        &self,
        listener: Arc<dyn StateMachineListener<S, E>>,
    ) -> Result<(), BoxError> {
        todo!()
    }

    /// Removes the state listener.
    fn remove_state_listener(
        &self,
        listener: &dyn StateMachineListener<S, E>,
    ) -> Result<(), BoxError> {
        todo!()
    }
}

impl<S, E> StateMachine<S, E> for BaseStateMachine<S, E>
where
    S: Send + Sync,
    E: Send + Sync,
{
    fn initial_state(&self) -> &dyn StateMachineState<S, E> {
        self.initial_state.as_ref()
    }

    /// Gets the state machine extended state.
    fn extended_state(&self) -> &dyn ExtendedState {
        self.extended_state.as_ref()
    }

    fn state_machine_accessor(&self) -> &dyn StateMachineAccessor<S, E> {
        todo!()
    }

    /// Sets the state machine error.
    fn set_state_machine_error(&mut self, error: Box<dyn std::error::Error + Send>) {
        todo!()
    }

    /// Checks for state machine error.
    fn has_state_machine_error(&self) -> bool {
        todo!()
    }
}

impl<S, E> StateMachineObjectSupportExt<S, E> for BaseStateMachine<S, E>
where
    S: 'static,
    E: 'static,
{
    fn notify_event_not_accepted(&self, state_context: &dyn StateContext<S, E>) {
        if self.parent_machine.is_none() {
            self.object_support.notify_event_not_accepted(state_context);
        }
    }
}

impl<S, E> LifecycleObjectSupportExt for BaseStateMachine<S, E>
where
    S: Clone,
    S: Eq + Hash,
    S: Send + Sync + 'static,
    E: Clone,
    E: Send + Sync + 'static,
{
    fn on_init(&mut self) -> Result<(), BoxError> {
        self.object_support.lifecycle_object_support.on_init();

        // Validate initial state
        assert!(
            self.initial_state.pseudo_state().is_some()
                && self.initial_state.pseudo_state().unwrap().get_kind()
                    == PseudoStateKind::Initial,
            "Initial state's pseudostate kind must be INITIAL"
        );

        self.last_state = None;

        struct _DefaultExtendedStateChangeListener<S, E>(
            StateMachineObjectSupport<S, E>,
            DefaultStateContext<S, E>,
        );

        #[async_trait]
        impl<S: Send + Sync + 'static, E: Send + Sync + 'static> ExtendedStateChangeListener
            for _DefaultExtendedStateChangeListener<S, E>
        {
            async fn changed(&self, key: &str, value: &AnyValue) {
                self.0
                    .notify_extended_state_changed(key, value, &self.1)
                    .await;
            }
        }

        let object_support = self.object_support.clone();
        let state_machine = self.get_relay_state_machine();
        let staete_context =
            self.build_state_context(Stage::ExtendedStateChanged, None, None, state_machine);

        self.extended_state
            .set_extended_state_change_listener(Arc::new(_DefaultExtendedStateChangeListener(
                object_support,
                staete_context,
            )));

        // Process transitions
        for transition in self.transitions.iter() {
            if let Some(trigger) = transition.trigger() {
                self.trigger_to_transition
                    .insert(trigger.id().to_string(), transition.clone());
            } else {
                self.triggerless_transitions.push(transition.clone());
            }
        }

        struct _DefaultStateListener<S, E>(
            Arc<dyn StateMachineState<S, E, DefaultStateMachineEventResult<S, E>>>,
            Arc<dyn StateMachine<S, E>>,
        );

        #[async_trait]
        impl<S: Send + Sync + 'static, E: Send + Sync + 'static> StateListener<S, E>
            for _DefaultStateListener<S, E>
        {
            async fn do_on_complete(&self, context: &dyn StateContext<S, E>) {
                #[cfg(feature = "trace-log")]
                tracing::debug!("State on_complete.");

                if let Some(state_machine) =
                    (self.1.as_ref() as &dyn Any).downcast_ref::<BaseStateMachine<S, E>>()
                {
                    state_machine
                        .execute_triggerless_transitions(context, self.0.as_ref())
                        .await;
                }
            }

            async fn on_entry(&self, _context: &dyn StateContext<S, E>) {}
            async fn on_exit(&self, _context: &dyn StateContext<S, E>) {}
            async fn on_complete(&self, _context: &dyn StateContext<S, E>) {}
        }

        for state in self.states.iter() {
            let state = state.clone();
            let state_machine = self.get_relay_state_machine();

            state.add_state_listener(Arc::new(_DefaultStateListener(state, state_machine)));

            if state.is_submachine_state() {
                if let Some(state) = (state.as_ref() as &dyn Any).downcast_ref::<BaseState<S, E>>()
                {
                    state.get_sub_machine().add_state_listener(Arc::new(
                        StateMachineListenerRelay::new(self.object_support.state_listener.clone()),
                    ));
                }
            } else if state.is_orthogonal() {
                if let Some(state) = (state.as_ref() as &dyn Any).downcast_ref::<BaseState<S, E>>()
                {
                    state.get_regions().iter().for_each(|region| {
                        let _ =
                            region.add_state_listener(Arc::new(StateMachineListenerRelay::new(
                                self.object_support.state_listener.clone(),
                            )));
                    });
                }
            }

            if let Some(pseudo_state) = state.pseudo_state() {
                if pseudo_state.get_kind() == PseudoStateKind::HistoryDeep {
                    self.history = pseudo_state.clone().into();
                }
            }
        }

        // Create executor
        // let executor = DefaultStateMachineExecutor::new(
        //     self,
        //     self.get_relay_state_machine(),
        //     self.transitions.clone(),
        //     trigger_to_transition,
        //     triggerless_transitions,
        //     self.initial_transition.clone(),
        //     self.initial_event.clone(),
        //     self.transition_conflict_policy,
        // );

        // self.state_machine_executor = Some(Box::new(executor));

        Ok(())
    }
}

impl<S, E> Clone for BaseStateMachine<S, E>
where
    S: Clone,
    S: Eq + Hash,
    S: Send + Sync + 'static,
    E: Clone,
    E: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        BaseStateMachine {
            states: self.states.clone(),
            transitions: self.transitions.clone(),
            initial_state: self.initial_state.clone(),
            initial_transition: self.initial_transition.clone(),
            initial_event: self.initial_event.clone(),
            extended_state: self.extended_state.clone(),
            transition_conflict_policy: self.transition_conflict_policy,
            current_state: self.current_state.clone(),
            last_state: self.last_state.clone(),
            current_error: todo!(),
            history: self.history.clone(),
            trigger_to_transition: self.trigger_to_transition.clone(),
            triggerless_transitions: self.triggerless_transitions.clone(),
            relay: self.relay.clone(),
            state_machine_executor: todo!(),
            initial_enabled: self.initial_enabled.clone(),
            uuid: self.uuid,
            id: self.id.clone(),
            forwarded_initial_event: self.forwarded_initial_event.clone(),
            parent_machine: self.parent_machine.clone(),

            object_support: Default::default(),
        }
    }
}

impl<S, E> fmt::Display for BaseStateMachine<S, E>
where
    S: Debug,
    S: Send + Sync,
    E: Send + Sync,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = String::new();
        for state in &self.states {
            for s in state.states() {
                buf.push_str(&format!("{:?} ", s.id()));
            }
        }
        buf.push_str(" / ");

        if let Some(current) = self.current_state.as_ref() {
            buf.push_str(&format!("{:?}", current.ids()));
        }

        buf.push_str(" / uuid=");
        buf.push_str(&self.uuid.to_string());
        buf.push_str(" / id=");
        if let Some(id) = self.id.as_ref() {
            buf.push_str(id);
        }

        write!(f, "{}", buf)
    }
}

impl<S, E> Deref for BaseStateMachine<S, E> {
    type Target = StateMachineObjectSupport<S, E>;
    fn deref(&self) -> &Self::Target {
        &self.object_support
    }
}

impl<S, E> DerefMut for BaseStateMachine<S, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_support
    }
}
