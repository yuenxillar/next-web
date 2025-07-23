use super::application_event::ApplicationEvent;
use async_trait::async_trait;
use next_web_core::DynClone;
use std::any::TypeId;

/// 应用事件监听器
/// Application event listener
#[async_trait]
pub trait ApplicationListener: DynClone + Send + Sync {
    
    /// 获取事件类型
    ///
    /// Get event type
    fn tid(&self) -> TypeId;

    /// 获取事件ID
    /// Get event ID
    fn id(&self) -> String {
        "".into()
    }

    /// 处理应用事件
    /// Handle application event
    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>);
}

next_web_core::clone_trait_object!(ApplicationListener);
