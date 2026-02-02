use crate::traits::event::application_event::EventId;

use super::application_event::ApplicationEvent;
use async_trait::async_trait;
use dyn_clone::DynClone;

/// 应用事件监听器
///
/// Application event listener
#[async_trait]
pub trait ApplicationListener
where
    Self: Send + Sync,
    Self: DynClone,
{
    /// 获取事件类型
    ///
    /// Get event typeid
    fn event_id(&self) -> EventId;

    /// 处理应用事件
    ///
    /// Handle application event
    async fn on_application_event(&self, event: &Box<dyn ApplicationEvent>);
}

dyn_clone::clone_trait_object!(ApplicationListener);
