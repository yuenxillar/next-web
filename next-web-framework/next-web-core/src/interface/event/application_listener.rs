use super::application_event::ApplicationEvent;
use async_trait::async_trait;
use dyn_clone::DynClone;
use std::any::TypeId;

/// 应用事件监听器
/// 
/// Application event listener
#[async_trait]
pub trait ApplicationListener: DynClone + Send + Sync {
    
    /// 获取事件ID
    /// 
    /// Get event ID
    fn id(&self) -> String;

    
    /// 获取事件类型
    ///
    /// Get event typeid
    fn event_id(&self) -> TypeId;


    /// 处理应用事件
    /// 
    /// Handle application event
    async fn on_application_event(&mut self, event: &Box<dyn ApplicationEvent>);
}

dyn_clone::clone_trait_object!(ApplicationListener);
