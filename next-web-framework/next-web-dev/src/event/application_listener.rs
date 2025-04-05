use std::{any::TypeId, borrow::Cow};
use async_trait::async_trait;
use super::application_event::ApplicationEvent;

/// 应用事件监听器
/// Application event listener
#[async_trait]
pub trait ApplicationListener: Send + Sync
{
    /// 获取事件类型
    /// Get event type
    fn tid(&self) -> TypeId;

    /// 获取事件ID
    /// Get event ID
    fn id(&self) -> Cow<'static, str> {
        "".into()
    }

    /// 处理应用事件
    /// Handle application event
    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>);
}
