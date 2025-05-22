use next_web_core::async_trait;

/// MQTT主题处理基础trait
/// Base trait for MQTT topic handling
///
/// 定义MQTT消息处理的基本接口，实现该trait的类型可以处理特定主题的消息
/// Defines the basic interface for MQTT message processing, types implementing this trait can handle messages for specific topics
#[async_trait]
pub trait BaseTopic: dyn_clone::DynClone + Send + Sync {
    /// 获取当前处理器订阅的主题
    /// Get the topic that this handler subscribes to
    ///
    /// # 返回值 Return value
    /// - &'static str: 订阅的主题字符串 / The subscribed topic string
    fn topic(&self) -> &'static str;

    /// 消费MQTT消息
    /// Consume MQTT message
    ///
    /// # 参数 Parameters
    /// - topic: 实际收到的主题 / The actual topic received
    /// - message: 消息内容 / Message content
    async fn consume(&self, topic: &str, message: &[u8]);
}

dyn_clone::clone_trait_object!(BaseTopic);
