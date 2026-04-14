use next_web_macros::properties;
use rudi_dev::singleton;

/// Kafka starter properties.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.mq.kafka")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct KafkaProperties {
    bootstrap_servers: Option<Vec<String>>,
    client_id: Option<String>,
    group_id: Option<String>,
    auto_create_topics: Option<bool>,
    auto_startup: Option<bool>,
    poll_timeout_ms: Option<u64>,
    request_timeout_ms: Option<u64>,
}

impl KafkaProperties {
    pub fn bootstrap_servers(&self) -> Vec<String> {
        self.bootstrap_servers
            .clone()
            .unwrap_or_else(|| vec!["127.0.0.1:9092".into()])
    }

    pub fn client_id(&self) -> &str {
        self.client_id.as_deref().unwrap_or("next-web")
    }

    pub fn group_id(&self) -> Option<&str> {
        self.group_id.as_deref()
    }

    pub fn auto_create_topics(&self) -> bool {
        self.auto_create_topics.unwrap_or(false)
    }

    pub fn auto_startup(&self) -> bool {
        self.auto_startup.unwrap_or(true)
    }

    pub fn poll_timeout_ms(&self) -> u64 {
        self.poll_timeout_ms.unwrap_or(1_000)
    }

    pub fn request_timeout_ms(&self) -> u64 {
        self.request_timeout_ms.unwrap_or(30_000)
    }
}

impl Default for KafkaProperties {
    fn default() -> Self {
        Self {
            bootstrap_servers: Some(vec!["127.0.0.1:9092".into()]),
            client_id: Some("next-web".into()),
            group_id: None,
            auto_create_topics: Some(false),
            auto_startup: Some(true),
            poll_timeout_ms: Some(1_000),
            request_timeout_ms: Some(30_000),
        }
    }
}
