use next_web_core::{
    async_trait,
    traits::event::{
        application_event::{ApplicationEvent, EventId},
        application_event_multicaster::ApplicationEventMulticaster,
        application_listener::ApplicationListener,
    },
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
#[cfg(feature = "trace-log")]
use tracing::debug;

use crate::util::thread::ThreadUtil;

type Listeners = Vec<Arc<dyn ApplicationListener>>;
/// 默认的事件多播器实现
///
/// Default implementation of event multicaster
#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    // 使用 (id, type_id) 存储不同类型事件的监听器
    //
    // Use (id, type_id) to store listeners for different event types
    application_listeners: Arc<Mutex<HashMap<EventId, Listeners>>>,

    /// 是否异步处理事件
    ///
    /// Whether to handle events asynchronously
    is_async: bool,
}

impl DefaultApplicationEventMulticaster {
    /// 创建新的事件多播器实例
    ///
    /// Create a new event multicaster instance
    pub fn new() -> Self {
        DefaultApplicationEventMulticaster {
            application_listeners: Arc::new(Mutex::new(HashMap::with_capacity(64))),
            is_async: true,
        }
    }

    /// 创建新的事件多播器实例，指定容量
    ///
    /// Create a new event multicaster instance with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        DefaultApplicationEventMulticaster {
            application_listeners: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
            is_async: true,
        }
    }

    /// 设置是否异步处理事件
    ///
    /// Set whether to handle events asynchronously
    pub fn set_async(&mut self, is_async: bool) {
        self.is_async = is_async;
    }
}

#[async_trait]
impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    async fn add_application_listener(&mut self, listener: Arc<dyn ApplicationListener>) {
        let event_id = listener.event_id();

        let mut application_listeners = self.application_listeners.lock().await;

        let listeners = application_listeners.entry(event_id.clone()).or_default();
        listeners.push(listener);

        #[cfg(feature = "trace-log")]
        {
            debug!(
                "Added listener for event type: {:?}, id: {}",
                event_id.0, event_id.1
            );
        }
    }

    async fn remove_application_listener(&mut self, id: &EventId) {
        // 使用监听器的唯一ID进行匹配删除
        // Use listener's unique ID to match and remove
        if let Some(_) = self.application_listeners.lock().await.remove(id) {
            #[cfg(feature = "trace-log")]
            debug!("Removed listener for event type: {}", key);
        };
    }

    /// 移除所有应用事件监听器
    ///
    /// Remove all application event listeners
    async fn remove_all_listeners(&mut self) {
        self.application_listeners.lock().await.clear();
    }

    /// 广播应用事件
    ///
    /// Multicast application event
    async fn multicast_event(&self, event: Box<dyn ApplicationEvent>) {
        let application_listeners = self.application_listeners.lock().await;

        match application_listeners.get(&event.event_id()) {
            Some(listeners) => {
                if self.is_async {
                    let listeners = listeners.clone();
                    ThreadUtil::spawn(async move {
                        for listener in listeners.iter() {
                            listener.on_application_event(&event).await;
                        }
                    });
                } else {
                    for listener in listeners.iter() {
                        listener.on_application_event(&event).await;
                    }
                }
            }
            None => {
                #[cfg(feature = "trace-log")]
                debug!("No listeners found for event type: {}", event.event_id());
            }
        }
    }
}
