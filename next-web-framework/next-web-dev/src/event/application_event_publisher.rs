use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use super::application_event::ApplicationEvent;

/// 应用事件发布者
/// Application event publisher
#[async_trait]
pub trait ApplicationEventPublisher: Send + Sync
{

    /// 获取事件通道
    /// Get event channel
    fn channel(&self) -> Option<&Sender<dyn ApplicationEvent>>;

    /// 发布事件
    /// Publish event
    async fn publish_event(&self, event: &dyn ApplicationEvent) -> Result<(), Box<dyn std::error::Error>> {
        let _ = if let Some(sender) = self.channel() {
            sender.send(event).await?
        };
        Ok(())
    }
}