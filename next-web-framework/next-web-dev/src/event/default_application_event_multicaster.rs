use super::key::Key;
use super::{
    application_event::ApplicationEvent,
    application_event_multicaster::ApplicationEventMulticaster,
    application_listener::ApplicationListener,
};
use hashbrown::HashMap;
use rudi::Singleton;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use tracing::{debug, info};

/// 默认的事件多播器实现
/// Default implementation of event multicaster
#[derive(Default)]
#[Singleton()]
pub struct DefaultApplicationEventMulticaster {
    // 使用 TypeId 存储不同类型事件的监听器
    // Use TypeId to store listeners for different event types
    listeners: HashMap<Key, Vec<Box<dyn ApplicationListener>>>,
    listeners_type: HashMap<TypeId, Vec<Box<dyn ApplicationListener>>>,
}

impl Clone for DefaultApplicationEventMulticaster {
    fn clone(&self) -> Self {
        // 创建新的空实例而不是尝试克隆监听器
        DefaultApplicationEventMulticaster::new()
    }
}

impl DefaultApplicationEventMulticaster {
    /// 创建新的事件多播器实例
    /// Create a new event multicaster instance
    pub fn new() -> Self {
        DefaultApplicationEventMulticaster {
            listeners: HashMap::new(),
            listeners_type: HashMap::new(),
        }
    }

    /// 按优先级对监听器进行排序
    /// Sort listeners by priority
    fn sort_listener(&mut self) {
        self.listeners
            .values_mut()
            .for_each(|itme| itme.sort_by(|a, b| a.order().cmp(&b.order())));

        self.listeners_type
            .values_mut()
            .for_each(|itme| itme.sort_by(|a, b| a.order().cmp(&b.order())));
    }
}

impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(&mut self, listener: Box<dyn ApplicationListener>) {
        let id = listener.id().clone();
        let tid = listener.tid().clone();

        if id.is_empty() {
            self.listeners_type
                .entry(tid.clone())
                .or_insert_with(Vec::new)
                .push(listener);
        } else {
            let key = Key::new(listener.id(), listener.tid());
            let listeners = self.listeners.entry(key).or_insert_with(Vec::new);
            listeners.push(listener);
        }
        debug!("Added listener for event type: {:?}, id: {}", tid, id);
    }

    fn remove_application_listener(&mut self, key: &Key) {
        // 使用监听器的唯一ID进行匹配删除
        // Use listener's unique ID to match and remove
        if let Some(_) = self.listeners.remove(key) {
            debug!("Removed listener for event type: {}", key);
        };
    }

    async fn multicast_event(&mut self, id: &Cow<'static, str>, event: &dyn ApplicationEvent) {
        info!("Multicasting event type: {:?}", event.type_id());

        if id.is_empty() {
            // 调用所有监听器处理事件
            // Invoke all listeners to handle the event
            if let Some(listeners) = self.listeners_type.get_mut(&event.type_id()) {
                for listener in listeners.iter_mut() {
                    listener.on_application_event(event).await;
                }
            }
        } else {
            // 获取该事件类型的所有监听器
            // Get all listeners for this event type
            if let Some(listeners) = self
                .listeners
                .get_mut(&Key::new(id.clone(), event.type_id()))
            {
                for listener in listeners.iter_mut() {
                    listener.on_application_event(event).await;
                }
            }
        }
    }
}

#[derive(Debug)]
struct TestEvent;

impl ApplicationEvent for TestEvent {
    fn source(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

struct TestListener;

impl TestListener {
    pub fn arc_self(self) -> std::sync::Arc<dyn ApplicationListener> {
        std::sync::Arc::new(self)
    }
}

#[async_trait::async_trait]
impl ApplicationListener for TestListener {
    fn tid(&self) -> TypeId {
        TypeId::of::<TestEvent>()
    }

    async fn on_application_event(&mut self, event: &dyn ApplicationEvent) {
        println!("TestListener received event: {:?}", event.get_timestamp());
    }
}
