use std::fmt::{Display, Formatter};

use next_web_core::{
    error::BoxError,
    traits::{service::Service, singleton::Singleton},
};

use crate::{
    autoconfigure::kafka_properties::KafkaProperties,
    config::default_kafka_listener_registration::DefaultKafkaListenerRegistration,
    core::record::ProducerRecord,
};

/// Spring-style producer facade exposed as the default injectable Kafka service.
#[derive(Clone)]
pub struct KafkaTemplate {
    properties: KafkaProperties,
    registrations: Vec<DefaultKafkaListenerRegistration>,
}

impl Singleton for KafkaTemplate {}
impl Service for KafkaTemplate {}

impl KafkaTemplate {
    pub fn new(
        properties: KafkaProperties,
        registrations: Vec<DefaultKafkaListenerRegistration>,
    ) -> Self {
        Self {
            properties,
            registrations,
        }
    }

    pub fn properties(&self) -> &KafkaProperties {
        &self.properties
    }

    pub fn registrations(&self) -> &[DefaultKafkaListenerRegistration] {
        self.registrations.as_slice()
    }

    pub fn listener_topics(&self) -> Vec<&str> {
        self.registrations
            .iter()
            .map(|registration| registration.endpoint().topic())
            .collect()
    }

    pub async fn send(&self, record: ProducerRecord) -> Result<(), BoxError> {
        Err(Box::new(KafkaTransportUnavailableError::new(format!(
            "Kafka transport is not bound yet, topic=`{}`. This starter currently provides \
             starter contracts and auto-configuration metadata. Bind a concrete Kafka client \
             adapter before sending messages.",
            record.topic()
        ))))
    }
}

#[derive(Debug)]
struct KafkaTransportUnavailableError {
    message: String,
}

impl KafkaTransportUnavailableError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for KafkaTransportUnavailableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for KafkaTransportUnavailableError {}
