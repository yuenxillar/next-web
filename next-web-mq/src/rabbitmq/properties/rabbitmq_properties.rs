use next_web_macros::Properties;
use serde::Deserialize;
use rudi_dev::Singleton;


#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.rabbitmq")]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RabbitMQClientProperties {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    virtual_host: Option<String>,
    manual_ack: Option<bool>,
}

impl RabbitMQClientProperties {
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn virtual_host(&self) -> Option<&str> {
        self.virtual_host.as_deref()
    }

    pub fn manual_ack(&self) -> bool {
        self.manual_ack.unwrap_or(false)
    }
}
