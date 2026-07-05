use super::application_event::ApplicationEvent;
use async_trait::async_trait;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// 应用事件发布者
///
/// Application event publisher
#[async_trait]
pub trait ApplicationEventPublisher
where
    Self: Send + Sync,
{
    /// 发布事件
    ///
    /// Publish event
    async fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), BoxError>;
}
