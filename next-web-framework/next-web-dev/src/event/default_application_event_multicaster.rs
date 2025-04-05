use super::key::Key;
use super::{
    application_event::ApplicationEvent,
    application_event_multicaster::ApplicationEventMulticaster,
    application_listener::ApplicationListener,
};

use flume::{Receiver, Sender};
use hashbrown::HashMap;
use parking_lot::Mutex;
use std::any::TypeId;
use std::borrow::Cow;
use std::sync::Arc;
use tracing::{debug, info};

/// 默认的事件多播器实现
/// Default implementation of event multicaster
///
#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    // 使用 TypeId 存储不同类型事件的监听器
    // Use TypeId to store listeners for different event types
    listeners: Arc<Mutex<HashMap<Key, Sender<Box<dyn ApplicationEvent>>>>>,
    listeners_type: Arc<Mutex<HashMap<TypeId, Sender<Box<dyn ApplicationEvent>>>>>,

    // 事件通道
    // Event channel
    event_channel: Option<Receiver<(Cow<'static, str>, Box<dyn ApplicationEvent>)>>,
}

impl DefaultApplicationEventMulticaster {
    /// 创建新的事件多播器实例
    /// Create a new event multicaster instance
    pub fn new() -> Self {
        DefaultApplicationEventMulticaster {
            listeners: Arc::new(Mutex::new(HashMap::new())),
            listeners_type: Arc::new(Mutex::new(HashMap::new())),

            event_channel: None,
        }
    }

    /// 设置事件通道
    /// Set event channel
    pub fn set_event_channel(
        &mut self,
        channel: Receiver<(Cow<'static, str>, Box<dyn ApplicationEvent>)>,
    ) {
        self.event_channel = Some(channel);
    }

    /// 运行事件多播器
    /// Run event multicaster
    pub fn run(&self) {
        let channel = self.event_channel.clone().unwrap();
        let listeners = self.listeners.clone();
        let listeners_type = self.listeners_type.clone();
        tokio::spawn(async move {
            while let Ok(event) = channel.recv() {
                if event.0.is_empty() {
                    if let Some(listeners) = listeners_type.lock().get(&event.1.tid()) {
                        let _ = listeners.send(event.1);
                    }
                } else {
                    if let Some(listeners) = listeners.lock().get(&Key::new(event.0, event.1.tid()))
                    {
                        let _ = listeners.send(event.1);
                    }
                }
            }
        });
    }
}

impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(&mut self, mut listener: Box<dyn ApplicationListener>) {
        let id = listener.id().clone();
        let tid = listener.tid().clone();

        let (sender, receiver) = flume::unbounded();
        if id.is_empty() {
            if let None = self.listeners_type.lock().get(&tid) {
                self.listeners_type.lock().insert(tid, sender);
            };
        } else {
            let key = Key::new(listener.id(), listener.tid());
            if let None = self.listeners_type.lock().get(&tid) {
                self.listeners.lock().insert(key, sender);
            };
        }
        tokio::spawn(async move {
            while let Ok(event) = receiver.recv() {
                listener.on_application_event(&event).await;
                info!("Received event: {:?}", event.source());
            }
        });
        debug!("Added listener for event type: {:?}, id: {}", tid, id);
    }

    fn remove_application_listener(&mut self, key: &Key) {
        // 使用监听器的唯一ID进行匹配删除
        // Use listener's unique ID to match and remove
        if let Some(_) = self.listeners.lock().remove(key) {
            debug!("Removed listener for event type: {}", key);
        };
    }

    // async fn multicast_event(&mut self, id: &Cow<'static, str>, event: &dyn ApplicationEvent) {

    //     if id.is_empty() {
    //         // 调用所有监听器处理事件
    //         // Invoke all listeners to handle the event
    //         if let Some(listeners) = self.listeners_type.get_mut(&event.type_id()) {
    //             for listener in listeners.iter_mut() {
    //                 listener.on_application_event(event).await;
    //             }
    //         }
    //     } else {
    //         // 获取该事件类型的所有监听器
    //         // Get all listeners for this event type
    //         if let Some(listeners) = self
    //             .listeners
    //             .get_mut(&Key::new(id.clone(), event.type_id()))
    //         {
    //             for listener in listeners.iter_mut() {
    //                 listener.on_application_event(event).await;
    //             }
    //         }
    //     }
    // }
}
