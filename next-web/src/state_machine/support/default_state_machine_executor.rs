use std::{
    any::Any,
    collections::HashMap,
    fmt,
    sync::{atomic::AtomicBool, Arc},
};

use next_web_core::{async_trait, error::BoxError, traits::message::Message};
use tokio::sync::broadcast::Sender;

use crate::state_machine::{
    support::{
        lifecycle_object_support::{LifecycleObjectSupport, LifecycleObjectSupportExt},
        state_machine_executor::{StateMachineExecutorCallback, StateMachineExecutorTransit},
        state_machine_interceptor_list::StateMachineInterceptorList,
        transition_comparator::TransitionComparator,
    },
    transition::{transition_conflict_policy::TransitionConflictPolicy, StateMachineTransition},
    trigger::{timer_trigger::TimerTrigger, trigger_listener::TriggerListener, Trigger},
    StateMachine,
};

const REACTOR_CONTEXT_TRIGGER_ERRORS: &str = "stateMachineTriggerErrors";

type TriggerToTransitionMap<S, E> = HashMap<
    String,
    (
        Arc<dyn Trigger<S, E>>,
        Arc<dyn StateMachineTransition<S, E>>,
    ),
>;

#[derive(Clone)]
pub struct DefaultStateMachineExecutor<S, E, T> {
    state_machine: T,
    relay_state_machine: Option<T>,
    trigger_to_transition_map: TriggerToTransitionMap<S, E>,
    triggerless_transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
    initial_transition: Arc<dyn StateMachineTransition<S, E>>,
    initial_event: Box<dyn Message<E>>,
    transition_comparator: TransitionComparator<S, E>,
    transition_conflict_policy: TransitionConflictPolicy,

    // final Queue<Message<E>> deferList = new ConcurrentLinkedQueue<Message<E>>();
    initial_handled: Arc<AtomicBool>,
    interceptors: StateMachineInterceptorList<S, E>,
    forwarded_initial_event: Option<Box<dyn Message<E>>>,
    queued_message: Option<Box<dyn Message<E>>>,
    state_machine_executor_transit: Option<Arc<dyn StateMachineExecutorTransit<S, E>>>,
    trigger_sender: Option<Sender<TriggerQueueItem<S, E>>>,
    lifecycle_object_support: LifecycleObjectSupport,
}

impl<S, E, T> DefaultStateMachineExecutor<S, E, T>
where
    T: StateMachine<S, E>,
    S: PartialEq,
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,

    S: Clone,
    E: Clone,
{
    pub fn new(
        state_machine: T,
        relay_state_machine: Option<T>,
        transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        trigger_to_transition_map: TriggerToTransitionMap<S, E>,
        triggerless_transitions: Vec<Arc<dyn StateMachineTransition<S, E>>>,
        initial_transition: Arc<dyn StateMachineTransition<S, E>>,
        initial_event: Box<dyn Message<E>>,
        transition_conflict_policy: TransitionConflictPolicy,
    ) -> Self {
        let _ransition_conflict_policy = transition_conflict_policy;
        let mut default_state_machine_executor = Self {
            state_machine,
            relay_state_machine,
            trigger_to_transition_map,
            triggerless_transitions,
            transitions,
            initial_transition,
            initial_event,
            transition_conflict_policy,
            transition_comparator: TransitionComparator::new(_ransition_conflict_policy),
            initial_handled: Arc::new(AtomicBool::default()),
            interceptors: StateMachineInterceptorList::default(),
            forwarded_initial_event: Default::default(),
            queued_message: Default::default(),
            state_machine_executor_transit: Default::default(),
            trigger_sender: Default::default(),
            lifecycle_object_support: Default::default(),
        };
        // anonymous transitions are fixed, sort those now
        default_state_machine_executor
            .triggerless_transitions
            .sort_by(|left, right| {
                default_state_machine_executor
                    .transition_comparator
                    .compare(left.as_ref(), right.as_ref())
            });
        default_state_machine_executor.register_trigger_listener();

        default_state_machine_executor
    }
}

impl<S, E, T> DefaultStateMachineExecutor<S, E, T>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    async fn register_trigger_listener(&mut self) {
        struct _DefaultTriggerListener<S, E>(
            Sender<TriggerQueueItem<S, E>>,
            Arc<dyn Trigger<S, E>>,
        );

        #[async_trait]
        impl<S: Send + Sync + 'static + Clone, E: Send + Sync + 'static + Clone> TriggerListener
            for _DefaultTriggerListener<S, E>
        {
            async fn triggered(&self) {
                self.0
                    .send(TriggerQueueItem::new(self.1.clone(), None, None, None));
            }
        }

        for trigger in self.trigger_to_transition_map.values_mut() {
            if let Some(timer_trigger) =
                (trigger.0.as_ref() as &dyn Any).downcast_ref::<TimerTrigger<S, E>>()
            {
                if let Some(sender) = self.trigger_sender.as_ref() {
                    timer_trigger.add_trigger_listener(Arc::new(_DefaultTriggerListener::<S, E>(
                        sender.clone(),
                        trigger.0.clone(),
                    )));
                }
            }
        }
    }
}
// impl<S, E> StateMachineExecutor<S, E> for DefaultStateMachineExecutor<S, E> {}

#[async_trait]
impl<S, E, T> LifecycleObjectSupportExt for DefaultStateMachineExecutor<S, E, T>
where
    S: Clone,
    S: Send + Sync + 'static,
    E: Clone,
    E: Send + Sync + 'static,
    T: StateMachine<S, E>,
{
    fn on_init(&mut self) -> Result<(), BoxError> {
        let (tx, rx) = tokio::sync::broadcast::channel(256);
        self.trigger_sender.replace(tx);

        // self.handle_trigger();

        Ok(())
    }

    async fn do_pre_start(&mut self) {
        self.start_triggers();
    }
}

impl<S, E, T> DefaultStateMachineExecutor<S, E, T>
where
    T: StateMachine<S, E>,
{
    fn handle_trigger(&self, queue_item: TriggerQueueItem<S, E>) {
        let current_state = self.state_machine.state();
        if let Some(state) = current_state.as_ref() {
            // tracing::debug!("Process trigger item {:?}", self);

            // queued message is kept on a class level order to let
            // triggerless transition to receive this message if it doesn't
            // kick in in this poll loop.
        }
        tokio::spawn(async move {});
    }

    fn start_triggers(&self) {
        // self.trigger_to_transition_map
        //     .values()
        //     .map(|val| val.0)
        //     .filter(predicate)
    }
}

/// 触发器队列项
pub struct TriggerQueueItem<S, E> {
    /// 触发器
    pub trigger: Arc<dyn Trigger<S, E>>,
    /// 消息
    pub message: Option<Box<dyn Message<E>>>,
    /// 执行器回调
    pub callback: Option<Arc<dyn StateMachineExecutorCallback>>,
    /// 触发器回调
    pub trigger_callback: Option<Arc<dyn StateMachineExecutorCallback>>,
}

impl<S, E> TriggerQueueItem<S, E>
where
    S: Send + Sync + 'static,
    E: Send + Sync + 'static + Clone,
{
    /// 创建新的触发器队列项
    pub fn new(
        trigger: Arc<dyn Trigger<S, E>>,
        message: Option<Box<dyn Message<E>>>,
        callback: Option<Arc<dyn StateMachineExecutorCallback>>,
        trigger_callback: Option<Arc<dyn StateMachineExecutorCallback>>,
    ) -> Self {
        Self {
            trigger,
            message,
            callback,
            trigger_callback,
        }
    }
}

impl<S, E> fmt::Display for TriggerQueueItem<S, E>
where
    E: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TriggerQueueItem [message={:?}, trigger={}]",
            self.message.as_ref().map(|s| s.get_payload()),
            "<trigger>" // 避免打印复杂的 trigger
        )
    }
}

impl<S, E> Clone for TriggerQueueItem<S, E>
where
    S: 'static,
    E: 'static + Clone,
{
    fn clone(&self) -> Self {
        Self {
            trigger: self.trigger.clone(),
            message: self.message.clone(),
            callback: self.callback.clone(),
            trigger_callback: self.trigger_callback.clone(),
        }
    }
}
