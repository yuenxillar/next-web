use next_web_core::async_trait;

/// Define the basic interface of MQTT message listener,
/// and implement the type of this trait to handle messages on specific topics
#[async_trait]
pub trait TopicListener
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Get the topic that this handler subscribes to
    ///
    /// 获取当前处理器订阅的主题
    fn topic(&self) -> &'static str;

    /// Process a message for the specified topic
    ///
    /// 处理指定主题的消息
    async fn on_message(&self, topic: &str, message: &[u8]);
}
