use flume::{Receiver, Sender};
use hashbrown::HashMap;
use next_web_core::{
    common::key::Key,
    interface::event::{
        application_event::ApplicationEvent,
        application_event_multicaster::ApplicationEventMulticaster,
        application_listener::ApplicationListener,
    },
};
use parking_lot::Mutex;
use std::sync::Arc;
#[cfg(feature = "trace-log")]
use tracing::debug;

/// 默认的事件多播器实现
///
/// Default implementation of event multicaster
#[derive(Clone)]
pub struct DefaultApplicationEventMulticaster {
    // 使用 TypeId 存储不同类型事件的监听器
    // Use TypeId to store listeners for different event types
    listeners: Arc<Mutex<HashMap<Key, Sender<Box<dyn ApplicationEvent>>>>>,

    // 事件通道
    // Event channel
    event_channel: Option<Receiver<(String, Box<dyn ApplicationEvent>)>>,
}

impl DefaultApplicationEventMulticaster {
    /// 创建新的事件多播器实例
    /// Create a new event multicaster instance
    pub fn new() -> Self {
        DefaultApplicationEventMulticaster {
            listeners: Arc::new(Mutex::new(HashMap::new())),
            event_channel: None,
        }
    }

    /// 设置事件通道
    /// Set event channel
    pub(crate) fn set_event_channel(
        &mut self,
        channel: Receiver<(String, Box<dyn ApplicationEvent>)>,
    ) {
        self.event_channel = Some(channel);
    }

    /// 运行事件多播器
    /// Run event multicaster
    pub fn run(&self) {
        let channel = self.event_channel.clone().unwrap();
        let listeners = self.listeners.clone();
        tokio::spawn(async move {
            while let Ok(event) = channel.recv() {
                if let Some(listeners) =
                    listeners.lock().get(&Key::new(event.0, event.1.event_id()))
                {
                    let _ = listeners.send(event.1);
                }
            }
        });
    }
}

impl ApplicationEventMulticaster for DefaultApplicationEventMulticaster {
    fn add_application_listener(&mut self, mut listener: Box<dyn ApplicationListener>) {
        let tid = listener.event_id();

        let (sender, receiver) = flume::unbounded();
        let key = Key::new(listener.id(), listener.event_id());
        let mut listeners = self.listeners.lock();
        if listeners.contains_key(&key) {
            panic!(
                "Listener already exists for event type: {:?}, key: {}",
                tid, key
            );
        }
        if let None = listeners.insert(key, sender) {
            tokio::spawn(async move {
                while let Ok(event) = receiver.recv() {
                    listener.on_application_event(&event).await;
                    #[cfg(feature = "trace-log")]
                    debug!("Received event: {:?}", event.source());
                }
            });
            #[cfg(feature = "trace-log")]
            {
                let id = listener.id();
                debug!("Added listener for event type: {:?}, id: {}", tid, id);
            }
        }
    }

    fn remove_application_listener(&mut self, key: &Key) {
        // 使用监听器的唯一ID进行匹配删除
        // Use listener's unique ID to match and remove
        if let Some(_) = self.listeners.lock().remove(key) {
            #[cfg(feature = "trace-log")]
            debug!("Removed listener for event type: {}", key);
        };
    }
}
