use std::collections::BTreeMap;

/// Simplified Kafka consumer record abstraction exposed by the starter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConsumerRecord {
    topic: String,
    key: Option<Vec<u8>>,
    payload: Option<Vec<u8>>,
    partition: i32,
    offset: i64,
    headers: BTreeMap<String, Vec<u8>>,
}

impl ConsumerRecord {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            ..Self::default()
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn key(&self) -> Option<&[u8]> {
        self.key.as_deref()
    }

    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }

    pub fn partition(&self) -> i32 {
        self.partition
    }

    pub fn offset(&self) -> i64 {
        self.offset
    }

    pub fn headers(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.headers
    }

    pub fn with_key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_payload(mut self, payload: impl Into<Vec<u8>>) -> Self {
        self.payload = Some(payload.into());
        self
    }

    pub fn with_partition(mut self, partition: i32) -> Self {
        self.partition = partition;
        self
    }

    pub fn with_offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Simplified Kafka producer record abstraction exposed by the starter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProducerRecord {
    topic: String,
    key: Option<Vec<u8>>,
    payload: Vec<u8>,
    partition: Option<i32>,
    headers: BTreeMap<String, Vec<u8>>,
}

impl ProducerRecord {
    pub fn new(topic: impl Into<String>, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            topic: topic.into(),
            payload: payload.into(),
            ..Self::default()
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn key(&self) -> Option<&[u8]> {
        self.key.as_deref()
    }

    pub fn payload(&self) -> &[u8] {
        self.payload.as_slice()
    }

    pub fn partition(&self) -> Option<i32> {
        self.partition
    }

    pub fn headers(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.headers
    }

    pub fn with_key(mut self, key: impl Into<Vec<u8>>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn with_partition(mut self, partition: i32) -> Self {
        self.partition = Some(partition);
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}
