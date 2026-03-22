//! Support and helper module for base state machine implementation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use next_web_core::anys::any_value::AnyValue;
use next_web_core::traits::message::Message;
use tracing::error;

use crate::state_machine::event::state_machine_event_publisher::StateMachineEventPublisher;
use crate::state_machine::listener::composite_state_machine_listener::CompositeStateMachineListener;
use crate::state_machine::listener::state_machine_listener::StateMachineListener;
use crate::state_machine::monitor::composite_state_machine_monitor::CompositeStateMachineMonitor;
use crate::state_machine::monitor::state_machine_monitor::StateMachineMonitor;
use crate::state_machine::processor::state_machine_handler_call_helper::StateMachineHandlerCallHelper;
use crate::state_machine::state::StateMachineState;
use crate::state_machine::state_context::StateContext;
use crate::state_machine::support::lifecycle_object_support::LifecycleObjectSupport;
use crate::state_machine::support::state_machine_interceptor_list::StateMachineInterceptorList;
use crate::state_machine::transition::StateMachineTransition;
use crate::state_machine::StateMachine;

/// Support and helper class for base state machine implementation.
///
/// This struct provides common functionality for state machine implementations,
/// including event publishing, listener management, and interceptor handling.
#[derive(Clone)]
pub struct StateMachineObjectSupport<S, E> {
    /// Composite state listener for managing multiple listeners
    pub(crate) state_listener: CompositeStateMachineListener<S, E>,

    /// Composite state machine monitor for monitoring state machine operations
    pub(crate) state_machine_monitor: CompositeStateMachineMonitor<S, E>,

    /// State machine event publisher for publishing events
    state_machine_event_publisher: Option<Arc<dyn StateMachineEventPublisher<S, E>>>,

    /// Flag indicating if context application events are enabled
    context_events_enabled: bool,

    /// List of interceptors for state machine operations
    pub(crate) interceptors: StateMachineInterceptorList<S, E>,

    /// Bean name for this instance
    bean_name: Option<String>,

    /// Flag indicating if handlers have been initialized
    handlers_initialized: Arc<AtomicBool>,

    /// State machine handler call helper for annotation handlers
    state_machine_handler_call_helper: StateMachineHandlerCallHelper<S, E>,

    pub(crate) lifecycle_object_support: LifecycleObjectSupport,
}

impl<S, E> StateMachineObjectSupport<S, E>
where
    S: 'static,
    E: 'static,
    // S: Clone + Send + Sync,
    // E: Clone + Send + Sync,
{
    /// Creates a new instance of StateMachineObjectSupport with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the state machine object support.
    pub async fn start(&mut self) {
        if !self.handlers_initialized.load(Ordering::SeqCst) {
            match self
                .state_machine_handler_call_helper
                .after_properties_set()
                .await
            {
                Ok(_) => {
                    self.handlers_initialized.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    error!("Unable to initialize annotation handlers: {}", e);
                    self.handlers_initialized.store(true, Ordering::SeqCst);
                }
            }
        }
    }

    /// Sets the bean name for this instance.
    pub fn set_bean_name(&mut self, name: impl Into<String>) {
        self.bean_name = Some(name.into());
    }

    /// Returns the bean name if set.
    pub fn get_bean_name(&self) -> Option<&String> {
        self.bean_name.as_ref()
    }

    /// Gets the state machine event publisher.
    pub fn get_state_machine_event_publisher(
        &self,
    ) -> Option<Arc<dyn StateMachineEventPublisher<S, E>>> {
        self.state_machine_event_publisher.clone()
    }

    /// Sets the state machine event publisher.
    pub fn set_state_machine_event_publisher(
        &mut self,
        publisher: Arc<dyn StateMachineEventPublisher<S, E>>,
    ) {
        self.state_machine_event_publisher = Some(publisher);
    }

    /// Sets whether context application events are enabled.
    pub fn set_context_events_enabled(&mut self, enabled: bool) {
        self.context_events_enabled = enabled;
    }

    /// Returns a reference to the state listener.
    pub fn get_state_listener(&self) -> &CompositeStateMachineListener<S, E> {
        &self.state_listener
    }

    /// Returns a mutable reference to the state listener.
    pub fn get_state_listener_mut(&mut self) -> &mut CompositeStateMachineListener<S, E> {
        &mut self.state_listener
    }

    /// Returns a reference to the state machine monitor.
    pub fn get_state_machine_monitor(&self) -> &CompositeStateMachineMonitor<S, E> {
        &self.state_machine_monitor
    }

    /// Returns a mutable reference to the state machine monitor.
    pub fn get_state_machine_monitor_mut(&mut self) -> &mut CompositeStateMachineMonitor<S, E> {
        &mut self.state_machine_monitor
    }

    /// Notifies listeners about state changed event.
    pub async fn notify_state_changed(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_changed(self.bean_name.as_deref(), state_context);
        // self.state_machine_handler_call_helper
        //     .call_on_state_changed(state_context.get_state_machine().get_id(), state_context);

        let source = state_context.source();
        let target = state_context.target();

        if !(target.is_some() && source.is_some()) {
            return;
        }

        let source = source.unwrap();
        let target = target.unwrap();

        self.state_listener.state_changed(source, target);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                {
                    publisher.publish_state_changed(self, source, target).await;
                }
            }
        }

        // warn!("Error during notify_state_changed: {}", e);
    }

    /// Notifies listeners about state entered event.
    pub async fn notify_state_entered(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_entry(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_state_entry(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let target = match state_context.target() {
            Some(target) => target,
            None => return,
        };
        self.state_listener.state_entered(target);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher.publish_state_entered(self, target).await;
            }
        }

        // warn!("Error during notify_state_entered: {}", e);
    }

    /// Notifies listeners about state exited event.
    pub async fn notify_state_exited(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_exit(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_state_exit(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let source = match state_context.source() {
            Some(source) => source,
            None => return,
        };
        self.state_listener.state_exited(source);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher.publish_state_exited(self, source).await;
            }
        }

        // warn!("Error during notify_state_exited: {}", e);
    }

    /// Notifies listeners about event not accepted event.
    pub async fn notify_event_not_accepted(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_event_not_accepted(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_event_not_accepted(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let message = match state_context.message() {
            Some(message) => message,
            None => return,
        };
        self.state_listener.event_not_accepted(message);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher.publish_event_not_accepted(self, message).await;
            }
        }

        // warn!("Error during notify_event_not_accepted: {}", e);
    }

    /// Notifies listeners about transition start event.
    pub async fn notify_transition_start(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_transition_start(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_transition_start(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let transition = match state_context.transition() {
            Some(transition) => transition,
            None => return,
        };
        self.state_listener.transition_started(transition);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher.publish_transition_start(self, transition).await;
            }
        }

        // warn!("Error during notify_transition_start: {}", e);
    }

    /// Notifies listeners about transition event.
    pub async fn notify_transition(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_transition(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_transition(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let transition = match state_context.transition() {
            Some(transition) => transition,
            None => return,
        };
        self.state_listener.transition(transition);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = &self.state_machine_event_publisher {
                publisher.publish_transition(self, transition).await;
            }
        }

        // warn!("Error during notify_transition: {}", e);
    }

    /// Notifies listeners about transition end event.
    pub async fn notify_transition_end(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_transition_end(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_transition_end(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let transition = match state_context.transition() {
            Some(transition) => transition,
            None => return,
        };
        self.state_listener.transition_ended(transition);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = &self.state_machine_event_publisher {
                publisher.publish_transition_end(self, transition).await;
            }
        }

        // warn!("Error during notify_transition_end: {}", e);
    }

    /// Notifies listeners about state machine started event.
    pub async fn notify_state_machine_started(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_start(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_start(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let state_machine = match state_context.state_machine() {
            Some(state_machine) => state_machine,
            None => return,
        };
        self.state_listener.state_machine_started(state_machine);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = &self.state_machine_event_publisher {
                publisher
                    .publish_state_machine_start(self, state_machine)
                    .await;
            }
        }

        // warn!("Error during notify_state_machine_started: {}", e);
    }

    /// Notifies listeners about state machine stopped event.
    pub async fn notify_state_machine_stopped(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_stop(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_stop(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let state_machine = match state_context.state_machine() {
            Some(state_machine) => state_machine,
            None => return,
        };
        self.state_listener.state_machine_stopped(state_machine);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher
                    .publish_state_machine_stop(self, state_machine)
                    .await;
            }
        }

        // warn!("Error during notify_state_machine_stopped: {}", e);
    }

    /// Notifies listeners about state machine error event.
    pub async fn notify_state_machine_error(&self, state_context: &dyn StateContext<S, E>) {
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_error(self.bean_name.as_deref(), state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_state_machine_error(state_context.get_state_machine().get_id(), state_context)
        //     .await?;

        let error = match state_context.error() {
            Some(error) => error,
            None => return,
        };
        let state_machine = match state_context.state_machine() {
            Some(state_machine) => state_machine,
            None => return,
        };

        self.state_listener
            .state_machine_error(state_machine, error.as_ref());
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher
                    .publish_state_machine_error(self, state_machine, error.as_ref())
                    .await;
            }
        }

        // warn!("Error during notify_state_machine_error: {}", e);
    }

    /// Notifies listeners about extended state changed event.
    pub async fn notify_extended_state_changed(
        &self,
        key: &str,
        value: &AnyValue,
        state_context: &dyn StateContext<S, E>,
    ) where
        S: 'static,
        E: 'static,
    {
        // self.state_machine_handler_call_helper
        //     .call_on_extended_state_changed(self.bean_name.as_deref(), key, value, state_context)
        //     .await?;
        // self.state_machine_handler_call_helper
        //     .call_on_extended_state_changed(
        //         state_context.get_state_machine().get_id(),
        //         key,
        //         value,
        //         state_context,
        //     )
        //     .await?;

        self.state_listener.extended_state_changed(key, value);
        self.state_listener.state_context(state_context);

        if self.context_events_enabled {
            if let Some(publisher) = self.state_machine_event_publisher.as_ref() {
                publisher
                    .publish_extended_state_changed(self, key, value)
                    .await;
            }
        }

        // warn!("Error during notify_extended_state_changed: {}", e);
    }

    /// Notifies monitors about transition event.
    pub async fn notify_transition_monitor(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        transition: &dyn StateMachineTransition<S, E>,
        duration: Duration,
    ) {
        self.state_machine_monitor
            .transition(state_machine, transition, duration)

        // warn!("Error during notify_transition_monitor: {}", e);
    }

    /// Notifies monitors about action event.
    pub async fn notify_action_monitor(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        action: &dyn Fn(&dyn StateContext<S, E>) -> BoxFuture<'static, ()>,
        duration: Duration,
    ) {
        self.state_machine_monitor
            .action(state_machine, action, duration);
        // warn!("Error during notify_action_monitor: {}", e);
    }

    /// Returns a reference to the state machine interceptors.
    pub fn get_state_machine_interceptors(&self) -> &StateMachineInterceptorList<S, E> {
        &self.interceptors
    }

    /// Returns a mutable reference to the state machine interceptors.
    pub fn get_state_machine_interceptors_mut(&mut self) -> &mut StateMachineInterceptorList<S, E> {
        &mut self.interceptors
    }
}

impl<S, E> Default for StateMachineObjectSupport<S, E> {
    fn default() -> Self {
        Self {
            state_listener: Default::default(),
            state_machine_monitor: Default::default(),
            state_machine_event_publisher: None,
            context_events_enabled: true,
            interceptors: Default::default(),
            bean_name: None,
            handlers_initialized: Arc::new(AtomicBool::default()),
            state_machine_handler_call_helper: Default::default(),

            lifecycle_object_support: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct StateMachineListenerRelay<S, E>(pub(crate) CompositeStateMachineListener<S, E>);

impl<S, E> StateMachineListenerRelay<S, E> {
    pub fn new(listener: CompositeStateMachineListener<S, E>) -> Self {
        Self(listener)
    }
}

impl<S, E> StateMachineListener<S, E> for StateMachineListenerRelay<S, E> {
    fn state_changed(&self, from: &dyn StateMachineState<S, E>, to: &dyn StateMachineState<S, E>) {
        self.0.state_changed(from, to);
    }

    fn state_entered(&self, state: &dyn StateMachineState<S, E>) {
        self.0.state_entered(state);
    }

    fn state_exited(&self, state: &dyn StateMachineState<S, E>) {
        self.0.state_exited(state);
    }

    fn event_not_accepted(&self, event: &dyn Message<E>) {
        self.0.event_not_accepted(event);
    }

    fn transition(&self, transition: &dyn StateMachineTransition<S, E>) {
        self.0.transition(transition);
    }

    fn transition_started(&self, transition: &dyn StateMachineTransition<S, E>) {
        self.0.transition_started(transition);
    }

    fn transition_ended(&self, transition: &dyn StateMachineTransition<S, E>) {
        self.0.transition_ended(transition);
    }

    fn state_machine_started(&self, state_machine: &dyn StateMachine<S, E>) {
        self.0.state_machine_started(state_machine);
    }

    fn state_machine_stopped(&self, state_machine: &dyn StateMachine<S, E>) {
        self.0.state_machine_stopped(state_machine);
    }

    fn state_machine_error(
        &self,
        state_machine: &dyn StateMachine<S, E>,
        error: &dyn std::error::Error,
    ) {
        self.0.state_machine_error(state_machine, error);
    }

    fn extended_state_changed(&self, key: &str, value: &AnyValue) {
        self.0.extended_state_changed(key, value);
    }

    fn state_context(&self, state_context: &dyn StateContext<S, E>) {
        self.0.state_context(state_context);
    }
}

pub trait StateMachineObjectSupportExt<S, E> {
    fn notify_event_not_accepted(&self, state_context: &dyn StateContext<S, E>);
}
