use std::fmt::{Display, Formatter};

use next_web_core::{
    error::BoxError,
    traits::{service::Service, singleton::Singleton},
};

use crate::{
    autoconfigure::rocketmq_properties::RocketmqProperties,
    config::default_rocketmq_listener_registration::DefaultRocketmqListenerRegistration,
    core::message::RocketmqMessage,
};

/// Spring-style producer facade exposed as the default injectable RocketMQ
/// service.
///
/// The template keeps the resolved starter properties and discovered listener
/// registrations together so higher-level modules can inspect how the starter
/// was assembled. The actual transport binding can be added later without
/// changing the application-facing API.
#[derive(Clone)]
pub struct RocketmqTemplate {
    properties: RocketmqProperties,
    registrations: Vec<DefaultRocketmqListenerRegistration>,
}

impl Singleton for RocketmqTemplate {}
impl Service for RocketmqTemplate {}

impl RocketmqTemplate {
    /// Creates a new template from resolved properties and listener
    /// registrations.
    pub fn new(
        properties: RocketmqProperties,
        registrations: Vec<DefaultRocketmqListenerRegistration>,
    ) -> Self {
        Self {
            properties,
            registrations,
        }
    }

    /// Returns the resolved RocketMQ starter properties.
    pub fn properties(&self) -> &RocketmqProperties {
        &self.properties
    }

    /// Returns all listener registrations discovered during startup.
    pub fn registrations(&self) -> &[DefaultRocketmqListenerRegistration] {
        self.registrations.as_slice()
    }

    /// Returns the topics currently registered for message consumption.
    pub fn listener_topics(&self) -> Vec<&str> {
        self.registrations
            .iter()
            .map(|registration| registration.endpoint().topic())
            .collect()
    }

    /// Sends a message through the template.
    ///
    /// At the moment the starter provides the public API surface and
    /// auto-configuration metadata, but leaves the concrete RocketMQ transport
    /// adapter to a later integration step.
    pub async fn send(&self, message: RocketmqMessage) -> Result<(), BoxError> {
        Err(Box::new(RocketmqTransportUnavailableError::new(format!(
            "RocketMQ transport is not bound yet, topic=`{}`. This starter currently provides \
             starter contracts and auto-configuration metadata. Bind a concrete RocketMQ client \
             adapter before sending messages.",
            message.topic()
        ))))
    }

    /// Convenience helper that creates and sends a message for the given topic.
    pub async fn send_to_topic(
        &self,
        topic: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Result<(), BoxError> {
        self.send(RocketmqMessage::new(topic, body)).await
    }
}

#[derive(Debug)]
struct RocketmqTransportUnavailableError {
    message: String,
}

impl RocketmqTransportUnavailableError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for RocketmqTransportUnavailableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RocketmqTransportUnavailableError {}
