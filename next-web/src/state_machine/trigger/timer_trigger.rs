use std::sync::Arc;
use std::time::Duration;

use next_web_core::async_trait;
use next_web_core::traits::id::Id;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;

use crate::state_machine::support::lifecycle_object_support::{
    LifecycleObjectSupport, LifecycleObjectSupportExt,
};
use crate::state_machine::trigger::composite_trigger_listener::CompositeTriggerListener;
use crate::state_machine::trigger::trigger_context::TriggerContext;
use crate::state_machine::trigger::trigger_listener::TriggerListener;
use crate::state_machine::trigger::Trigger;

/// 定时器触发器
#[derive(Clone)]
pub struct TimerTrigger<S, E> {
    // 触发器监听器
    trigger_listener: Arc<Mutex<CompositeTriggerListener>>,
    // 周期（毫秒）
    period: u64,
    // 触发次数（0表示无限）
    count: usize,
    // 当前的任务句柄
    task_handle: Option<Arc<JoinHandle<()>>>,
    // 生命周期支持
    lifecycle: LifecycleObjectSupport,

    _marker: std::marker::PhantomData<(S, E)>,
}

impl<S, E> TimerTrigger<S, E>
where
    Self: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    /// 创建新的定时器触发器
    pub fn new(period: u64) -> Self {
        Self::with_count(period, 0)
    }

    /// 创建指定触发次数的定时器触发器
    pub fn with_count(period: u64, count: usize) -> Self {
        Self {
            period,
            count,
            trigger_listener: Arc::new(Mutex::new(CompositeTriggerListener::default())),
            task_handle: None,
            _marker: std::marker::PhantomData,
            lifecycle: Default::default(),
        }
    }

    /// 获取周期
    pub fn get_period(&self) -> u64 {
        self.period
    }

    /// 获取触发次数
    pub fn get_count(&self) -> usize {
        self.count
    }

    /// 调度定时任务
    fn schedule(&mut self) {
        let period = self.period;
        let count = self.count;
        let trigger_listener = self.trigger_listener.clone();

        let initial_delay = if count > 0 { period } else { 0 };

        // 创建异步任务
        let handle = tokio::spawn(async move {
            // 创建间隔流
            let mut interval = time::interval(Duration::from_millis(period));

            // 处理第一次延迟
            if initial_delay > 0 {
                time::sleep(Duration::from_millis(initial_delay)).await;
            }

            let mut triggered_count = 0;

            loop {
                interval.tick().await;

                // 通知触发
                trigger_listener.lock().await.triggered().await;

                triggered_count += 1;

                // 如果达到指定次数，退出循环
                if count > 0 && triggered_count >= count {
                    break;
                }
            }

            // 清理任务句柄
            // let mut handle_guard = task_handle.lock().await;
            // *handle_guard = None;
        });

        // 保存任务句柄
        let _ = self.task_handle.replace(Arc::new(handle));
    }

    /// 取消定时任务
    fn cancel(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }

    /// 通知触发
    async fn notify_triggered(&self) {
        self.trigger_listener.lock().await.triggered().await;
    }
}

impl<S, E> Id for TimerTrigger<S, E> {
    fn id(&self) -> &str {
        "timerTrigger"
    }
}

#[async_trait]
impl<S, E> Trigger<S, E> for TimerTrigger<S, E>
where
    Self: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    async fn evaluate(&self, _context: &dyn TriggerContext<S, E>) -> bool {
        false
    }

    async fn add_trigger_listener(&self, listener: Arc<dyn TriggerListener>) {
        self.trigger_listener
            .lock()
            .await
            .get_mut_listeners()
            .ordered
            .push(listener);
    }

    fn event(&self) -> Option<&E> {
        None
    }

    async fn arm(&mut self) {
        if self.task_handle.is_some() {
            return;
        }

        self.schedule();
    }

    async fn disarm(&mut self) {
        if self.count > 0 {
            self.cancel();
        }
    }
}

// 生命周期支持
#[async_trait]
impl<S, E> LifecycleObjectSupportExt for TimerTrigger<S, E>
where
    Self: Send + Sync + 'static,
    S: Clone,
    E: Clone,
{
    /// 启动前的准备工作
    async fn do_pre_start(&mut self) {
        if self.count > 0 {
            return;
        }
        self.schedule();
    }

    /// 停止前的清理工作
    async fn do_pre_stop(&mut self) {
        self.cancel();
    }
}
