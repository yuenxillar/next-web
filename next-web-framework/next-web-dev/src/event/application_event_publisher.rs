use std::borrow::Cow;

use super::application_event::ApplicationEvent;

/// 应用事件发布者
/// Application event publisher

pub(crate) trait ApplicationEventPublisher: Send + Sync
{
    fn id(&self) -> Cow<'static, str> {
        "".into()
    }

    /// 发布事件
    /// Publish event
    fn publish_event(&self, event: Box<dyn ApplicationEvent>) -> Result<(), Box<dyn std::error::Error>>;
}