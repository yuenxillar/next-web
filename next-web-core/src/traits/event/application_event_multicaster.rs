use std::sync::Arc;

use async_trait::async_trait;

use super::application_listener::ApplicationListener;
use crate::traits::event::application_event::ApplicationEvent;
use crate::traits::event::application_event::EventId;

/// 应用事件多播器
///
/// Application event multicaster
#[async_trait]
pub trait ApplicationEventMulticaster: Send + Sync {
    /// 添加应用事件监听器
    ///
    /// Add application event listener
    async fn add_application_listener(&mut self, listener: Arc<dyn ApplicationListener>);

    /// 移除应用事件监听器
    ///
    /// Remove application event listener
    async fn remove_application_listener(&mut self, id: &EventId);

    /// 移除所有应用事件监听器
    ///
    /// Remove all application event listeners
    async fn remove_all_listeners(&mut self);

    /// 广播应用事件
    ///
    /// Multicast application event
    async fn multicast_event(&self, event: Box<dyn ApplicationEvent>);
}
