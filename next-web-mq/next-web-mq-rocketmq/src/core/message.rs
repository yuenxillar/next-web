use std::collections::BTreeMap;

/// Simplified RocketMQ producer message abstraction exposed by the starter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RocketmqMessage {
    topic: String,
    body: Vec<u8>,
    tag: Option<String>,
    keys: Vec<String>,
    delay_level: Option<u32>,
    sharding_key: Option<String>,
    headers: BTreeMap<String, Vec<u8>>,
}

impl RocketmqMessage {
    /// Creates a new producer message for the given topic and body.
    pub fn new(topic: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            topic: topic.into(),
            body: body.into(),
            ..Self::default()
        }
    }

    /// Returns the topic name.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the raw message body.
    pub fn body(&self) -> &[u8] {
        self.body.as_slice()
    }

    /// Returns the optional RocketMQ tag.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Returns the configured business keys.
    pub fn keys(&self) -> &[String] {
        self.keys.as_slice()
    }

    /// Returns the optional delay level.
    pub fn delay_level(&self) -> Option<u32> {
        self.delay_level
    }

    /// Returns the optional sharding key used for orderly messages.
    pub fn sharding_key(&self) -> Option<&str> {
        self.sharding_key.as_deref()
    }

    /// Returns user-defined headers attached to the message.
    pub fn headers(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.headers
    }

    /// Sets the RocketMQ tag and returns the updated message.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Adds a business key and returns the updated message.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.keys.push(key.into());
        self
    }

    /// Sets the delay level and returns the updated message.
    pub fn with_delay_level(mut self, delay_level: u32) -> Self {
        self.delay_level = Some(delay_level.max(1));
        self
    }

    /// Sets the sharding key and returns the updated message.
    pub fn with_sharding_key(mut self, sharding_key: impl Into<String>) -> Self {
        self.sharding_key = Some(sharding_key.into());
        self
    }

    /// Adds a custom header and returns the updated message.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Simplified RocketMQ consumer delivery exposed to listener callbacks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RocketmqDelivery {
    message_id: Option<String>,
    topic: String,
    body: Vec<u8>,
    tag: Option<String>,
    keys: Vec<String>,
    queue_id: Option<i32>,
    reconsume_times: u32,
    headers: BTreeMap<String, Vec<u8>>,
}

impl RocketmqDelivery {
    /// Creates a new delivery envelope for the given topic and body.
    pub fn new(topic: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            topic: topic.into(),
            body: body.into(),
            ..Self::default()
        }
    }

    /// Returns the transport-level message identifier, if available.
    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    /// Returns the topic name.
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Returns the raw message body.
    pub fn body(&self) -> &[u8] {
        self.body.as_slice()
    }

    /// Returns the optional tag.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Returns the business keys.
    pub fn keys(&self) -> &[String] {
        self.keys.as_slice()
    }

    /// Returns the source queue identifier, if available.
    pub fn queue_id(&self) -> Option<i32> {
        self.queue_id
    }

    /// Returns the current reconsume count.
    pub fn reconsume_times(&self) -> u32 {
        self.reconsume_times
    }

    /// Returns custom headers attached to the message.
    pub fn headers(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.headers
    }

    /// Sets the transport-level message identifier and returns the updated
    /// delivery.
    pub fn with_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.message_id = Some(message_id.into());
        self
    }

    /// Sets the RocketMQ tag and returns the updated delivery.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Adds a business key and returns the updated delivery.
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.keys.push(key.into());
        self
    }

    /// Sets the source queue identifier and returns the updated delivery.
    pub fn with_queue_id(mut self, queue_id: i32) -> Self {
        self.queue_id = Some(queue_id);
        self
    }

    /// Sets the reconsume count and returns the updated delivery.
    pub fn with_reconsume_times(mut self, reconsume_times: u32) -> Self {
        self.reconsume_times = reconsume_times;
        self
    }

    /// Adds a custom header and returns the updated delivery.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}
