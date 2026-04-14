use next_web_macros::properties;
use rudi_dev::singleton;

/// RocketMQ starter properties.
///
/// These values are bound from the `next.mq.rocketmq` prefix and describe the
/// common defaults shared by producer and consumer flows.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.mq.rocketmq")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RocketmqProperties {
    name_servers: Option<Vec<String>>,
    namespace: Option<String>,
    producer_group: Option<String>,
    consumer_group: Option<String>,
    instance_name: Option<String>,
    topic_prefix: Option<String>,
    send_timeout_ms: Option<u64>,
    consume_timeout_ms: Option<u64>,
    retry_times: Option<u32>,
    consume_batch_size: Option<u32>,
    enable_message_trace: Option<bool>,
    vip_channel_enabled: Option<bool>,
    auto_startup: Option<bool>,
}

impl RocketmqProperties {
    /// Returns the configured RocketMQ name servers.
    ///
    /// When no explicit value is supplied, the starter defaults to a single
    /// local server entry.
    pub fn name_servers(&self) -> Vec<String> {
        self.name_servers
            .clone()
            .unwrap_or_else(|| vec!["127.0.0.1:9876".into()])
    }

    /// Returns the optional namespace applied to produced and consumed
    /// resources.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Returns the default producer group name.
    pub fn producer_group(&self) -> &str {
        self.producer_group
            .as_deref()
            .unwrap_or("next-web-producer")
    }

    /// Returns the default consumer group name.
    pub fn consumer_group(&self) -> &str {
        self.consumer_group
            .as_deref()
            .unwrap_or("next-web-consumer")
    }

    /// Returns the client instance name used when the underlying transport
    /// builds a producer or consumer.
    pub fn instance_name(&self) -> &str {
        self.instance_name.as_deref().unwrap_or("next-web")
    }

    /// Returns the optional topic prefix that application code can reuse to
    /// group topics by service.
    pub fn topic_prefix(&self) -> Option<&str> {
        self.topic_prefix.as_deref()
    }

    /// Returns the default send timeout in milliseconds.
    pub fn send_timeout_ms(&self) -> u64 {
        self.send_timeout_ms.unwrap_or(3_000)
    }

    /// Returns the default consume timeout in milliseconds.
    pub fn consume_timeout_ms(&self) -> u64 {
        self.consume_timeout_ms.unwrap_or(15_000)
    }

    /// Returns the number of times the client should retry failed sends.
    pub fn retry_times(&self) -> u32 {
        self.retry_times.unwrap_or(2)
    }

    /// Returns the maximum batch size passed to a consumer callback.
    pub fn consume_batch_size(&self) -> u32 {
        self.consume_batch_size.unwrap_or(1)
    }

    /// Returns whether message trace is enabled by default.
    pub fn enable_message_trace(&self) -> bool {
        self.enable_message_trace.unwrap_or(false)
    }

    /// Returns whether the VIP channel is enabled.
    pub fn vip_channel_enabled(&self) -> bool {
        self.vip_channel_enabled.unwrap_or(false)
    }

    /// Returns whether listener containers should start automatically.
    pub fn auto_startup(&self) -> bool {
        self.auto_startup.unwrap_or(true)
    }
}

impl Default for RocketmqProperties {
    fn default() -> Self {
        Self {
            name_servers: Some(vec!["127.0.0.1:9876".into()]),
            namespace: None,
            producer_group: Some("next-web-producer".into()),
            consumer_group: Some("next-web-consumer".into()),
            instance_name: Some("next-web".into()),
            topic_prefix: None,
            send_timeout_ms: Some(3_000),
            consume_timeout_ms: Some(15_000),
            retry_times: Some(2),
            consume_batch_size: Some(1),
            enable_message_trace: Some(false),
            vip_channel_enabled: Some(false),
            auto_startup: Some(true),
        }
    }
}
