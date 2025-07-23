use next_web_core::error::BoxError;

use super::application_event::ApplicationEvent;

/// 应用事件发布者
/// 
/// Application event publisher

pub trait ApplicationEventPublisher: Send + Sync {
    /// 发布事件
    /// Publish event
    fn publish_event(
        &self,
        id: impl Into<String>,
        event: Box<dyn ApplicationEvent>,
    ) -> Result<(), BoxError>;
}
