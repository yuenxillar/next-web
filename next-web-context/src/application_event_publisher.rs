use super::application_event::ApplicationEvent;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// 应用事件发布者
///
/// Application event publisher
pub trait ApplicationEventPublisher
where
    Self: Send + Sync,
{
    /// 发布事件
    ///
    /// Publish event
    fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), BoxError>;
}
