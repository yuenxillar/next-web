use next_web_core::async_trait;

/// TriggerListener for listening a Trigger events.

#[async_trait]
pub trait TriggerListener
where
    Self: Send + Sync,
{
    /// Notified when trigger has been triggered.
    async fn triggered(&self);
}
