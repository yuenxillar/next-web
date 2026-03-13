use next_web_core::{
    async_trait,
    traits::{
        any_clone::AnyClone,
        event::{
            application_event::ApplicationEvent,
            application_event_multicaster::{ApplicationEventMulticaster, MulticastError},
            application_listener::ApplicationListener,
        },
    },
};
use std::{any::Any, sync::Arc};
use std::{any::TypeId, collections::HashMap, time::Duration};
use tokio::sync::mpsc::{channel, Sender};
#[cfg(feature = "trace-log")]
use tracing::{debug, error, info, warn};

use crate::util::thread::ThreadUtil;

/// Event value types that can be sent through the channel
///
/// 可通过通道发送的事件值类型
pub enum EventValue {
    /// An actual event to be processed
    ///
    /// 待处理的实际事件
    Value(Box<dyn Any + Send>),

    /// A new listener to add
    ///
    /// 要添加的新监听器
    AddListener(Box<dyn AnyClone>),

    /// A listener to remove by its ID
    ///
    /// 要通过ID移除的监听器
    RemoveListener(String),

    /// Remove all listeners
    ///
    /// 移除所有监听器
    RemoveAllListener,

    /// Shutdown the event processor
    ///
    /// 关闭事件处理器
    Shutdown,
}

/// Default implementation of ApplicationEventMulticaster
///
/// ApplicationEventMulticaster的默认实现
#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    /// Map of event type IDs to their corresponding event channels
    ///
    /// 事件类型ID到对应事件通道的映射
    listeners: HashMap<TypeId, Sender<EventValue>>,
}

impl DefaultApplicationEventMulticaster {
    /// Creates a new empty multicaster
    ///
    /// 创建一个新的空多播器
    pub fn new() -> Self {
        Self {
            listeners: Default::default(),
        }
    }

    /// Creates a new multicaster with the specified initial capacity
    ///
    /// 使用指定的初始容量创建一个新的多播器
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            listeners: HashMap::with_capacity(capacity),
        }
    }

    /// Returns the number of event types that have listeners
    ///
    /// 返回有监听器的事件类型数量
    pub fn event_type_count(&self) -> usize {
        self.listeners.len()
    }

    /// Removes and shuts down an event processor for a specific event type
    ///
    /// 移除并关闭特定事件类型的事件处理器
    pub async fn remove_event_processor<E: ApplicationEvent>(&mut self) {
        if let Some(sender) = self.listeners.remove(&TypeId::of::<E>()) {
            let _ = sender.send(EventValue::Shutdown).await;

            #[cfg(feature = "trace-log")]
            info!(
                "Removed event processor for type: {}",
                std::any::type_name::<E>()
            );
        }
    }
}

impl Default for DefaultApplicationEventMulticaster {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    /// Adds an application listener for a specific event type
    ///
    /// 为特定事件类型添加应用监听器
    async fn add_application_listener<L, E>(&mut self, id: String, listener: L)
    where
        L: ApplicationListener<E>,
        E: ApplicationEvent,
    {
        let event_type_id = TypeId::of::<E>();

        let sender = self.listeners.entry(event_type_id).or_insert_with(|| {
            let (tx, mut rx) = channel::<EventValue>(100);

            #[cfg(feature = "trace-log")]
            let event_type_name = std::any::type_name::<E>();

            let id = id.clone();
            // Spawn event processor actor
            //
            // 启动事件处理器actor
            ThreadUtil::spawn(async move {
                #[cfg(feature = "trace-log")]
                debug!("Starting event processor for type: {}", event_type_name);

                let mut listeners: HashMap<String, Arc<dyn ApplicationListener<E>>> =
                    HashMap::new();
                let mut shutdown = false;

                while let Some(event_value) = rx.recv().await {
                    if shutdown {
                        break;
                    }

                    match event_value {
                        EventValue::Value(box_event) => {
                            if let Some(event) = (box_event.as_ref()).downcast_ref::<E>() {
                                for listener in listeners.values() {
                                    listener.on_application_event(event).await;
                                }
                            } else {
                                #[cfg(feature = "trace-log")]
                                warn!(
                                    "Received event of wrong type for processor: {}",
                                    event_type_name
                                );
                            }
                        }
                        EventValue::AddListener(listener_arc) => {
                            if let Ok(listener) = listener_arc
                                .into_any()
                                .downcast::<Arc<dyn ApplicationListener<E>>>()
                            {
                                listeners.insert(id.clone(), *listener);

                                #[cfg(feature = "trace-log")]
                                debug!(
                                    "Listener added: {}, total listeners: {}",
                                    id,
                                    listeners.len()
                                );
                            } else {
                                #[cfg(feature = "trace-log")]
                                error!(
                                    "Failed to downcast listener for event type: {}",
                                    event_type_name
                                );
                            }
                        }
                        EventValue::RemoveListener(id) => {
                            if listeners.remove(&id).is_some() {
                                #[cfg(feature = "trace-log")]
                                info!(
                                    "Listener removed: {}, remaining listeners: {}",
                                    id,
                                    listeners.len()
                                );
                            } else {
                                #[cfg(feature = "trace-log")]
                                warn!("Attempted to remove non-existent listener: {}", id);
                            }
                        }
                        EventValue::RemoveAllListener => {
                            listeners.clear();
                        }
                        EventValue::Shutdown => {
                            shutdown = true;
                        }
                    }
                }

                #[cfg(feature = "trace-log")]
                info!("Event processor shutdown complete");
            });

            tx
        });

        // Send add listener message
        //
        // 发送添加监听器消息
        match sender
            .send(EventValue::AddListener(Box::new(
                Arc::new(listener) as Arc<dyn ApplicationListener<E>>
            )))
            .await
        {
            Ok(_) => {}
            #[allow(unused_variables)]
            Err(e) => {
                #[cfg(feature = "trace-log")]
                error!(
                    "Failed to add listener {} for event type {}: {}",
                    id,
                    std::any::type_name::<E>(),
                    e
                );

                // Clean up empty channel if send failed
                // 如果发送失败，清理空的channel
                if let Some(sender) = self.listeners.get(&event_type_id) {
                    if sender.is_closed() {
                        self.listeners.remove(&event_type_id);

                        #[cfg(feature = "trace-log")]
                        info!(
                            "Removed empty event processor for type: {}",
                            std::any::type_name::<E>()
                        );
                    }
                }
            }
        }
    }

    /// Removes an application listener for a specific event type
    ///
    /// 移除特定事件类型的应用监听器
    async fn remove_application_listener<E>(&mut self, listener_id: String)
    where
        E: ApplicationEvent,
    {
        if let Some(sender) = self.listeners.get_mut(&TypeId::of::<E>()) {
            match sender
                .send(EventValue::RemoveListener(listener_id.clone()))
                .await
            {
                Ok(_) => {
                    #[cfg(feature = "trace-log")]
                    info!(
                        "Successfully removed listener {} for event type {}",
                        listener_id,
                        std::any::type_name::<E>()
                    );
                }
                #[allow(unused_variables)]
                Err(e) => {
                    #[cfg(feature = "trace-log")]
                    error!(
                        "Failed to remove listener {} for event type {}: {}",
                        listener_id,
                        std::any::type_name::<E>(),
                        e
                    );
                }
            }
        } else {
            #[cfg(feature = "trace-log")]
            warn!(
                "Attempted to remove listener for event type {} with no registered listeners",
                std::any::type_name::<E>()
            );
        }
    }

    /// Removes all application listeners for all event types
    ///
    /// 移除所有事件类型的所有应用监听器
    async fn remove_all_listeners(&mut self) {
        // Send shutdown to all processors
        // 向所有处理器发送关闭信号
        for (_, sender) in self.listeners.drain() {
            let _ = sender.send(EventValue::RemoveAllListener).await;
        }
    }

    /// Multicasts an event to all registered listeners
    ///
    /// 向所有注册的监听器广播事件
    async fn multicast_event<E>(&self, event: E) -> Result<(), MulticastError>
    where
        E: ApplicationEvent,
    {
        let event_type_id = TypeId::of::<E>();

        match self.listeners.get(&event_type_id) {
            Some(sender) => {
                match sender
                    .send_timeout(
                        EventValue::Value(Box::new(event)),
                        Duration::from_millis(500),
                    )
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        #[cfg(feature = "trace-log")]
                        error!(
                            "Failed to send event of type {}: {}",
                            std::any::type_name::<E>(),
                            e
                        );

                        Err(MulticastError::SendError(format!(
                            "Failed to send event: {}",
                            e
                        )))
                    }
                }
            }
            None => {
                #[cfg(feature = "trace-log")]
                debug!(
                    "No listeners found for event type {}",
                    std::any::type_name::<E>()
                );

                // Return Ok even if no listeners, as this is not an error condition
                // 即使没有监听器也返回Ok，因为这不是错误情况
                Ok(())
            }
        }
    }
}
