use std::{any::TypeId, borrow::Cow};
use async_trait::async_trait;
use super::application_event::ApplicationEvent;

/// 应用事件监听器
/// Application event listener
#[async_trait]
pub trait ApplicationListener: Send + Sync
{
    fn tid(&self) -> TypeId;

    // 获取监听器顺序
    // Get listener order
    fn order(&self) -> i32 {
        i32::MAX
    }

    /// 获取事件ID
    /// Get event ID
    fn id(&self) -> Cow<'static, str> {
        "".into()
    }

    /// 处理应用事件
    /// Handle application event
    async fn on_application_event(&mut self, event: &dyn ApplicationEvent);
}
