use async_trait::async_trait;

use crate::error::BoxError;

use super::application_event::ApplicationEvent;

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
    async fn publish_event<E>(&self, event: E) -> Result<(), BoxError>
    where
        E: ApplicationEvent;
}
