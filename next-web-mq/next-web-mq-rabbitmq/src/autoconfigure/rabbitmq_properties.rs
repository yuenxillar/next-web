use next_web_macros::properties;
use rudi_dev::singleton;

/// RabbitMQ client properties.
#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.mq.rabbitmq")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RabbitmqProperties {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    virtual_host: Option<String>,
    manual_ack: Option<bool>,
    requeue_rejected: Option<bool>,
}

impl RabbitmqProperties {
    pub fn host(&self) -> &str {
        self.host.as_deref().unwrap_or("localhost")
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(5672)
    }

    pub fn username(&self) -> &str {
        self.username.as_deref().unwrap_or("guest")
    }

    pub fn password(&self) -> &str {
        self.password.as_deref().unwrap_or("guest")
    }

    pub fn virtual_host(&self) -> &str {
        self.virtual_host.as_deref().unwrap_or("/")
    }

    pub fn manual_ack(&self) -> bool {
        self.manual_ack.unwrap_or(false)
    }

    pub fn requeue_rejected(&self) -> bool {
        self.requeue_rejected.unwrap_or(true)
    }
}

impl Default for RabbitmqProperties {
    fn default() -> Self {
        Self {
            host: Some("localhost".into()),
            port: Some(5672),
            username: Some("guest".into()),
            password: Some("guest".into()),
            virtual_host: Some("/".into()),
            manual_ack: Some(false),
            requeue_rejected: Some(true),
        }
    }
}
