use std::collections::HashMap;

use next_web_core::server::ssl::Ssl;
use next_web_macros::properties;
use rudi_dev::singleton;

#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.mail")]
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct MailProperties {
    /// SMTP server host. For instance, 'smtp.example.com'.
    pub host: Option<String>,

    /// SMTP server port.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Login user of the SMTP server.
    pub username: Option<String>,

    /// Login password of the SMTP server.
    pub password: Option<String>,

    /// Protocol used by the SMTP server.
    #[serde(default = "default_protocol")]
    pub protocol: String,

    /// Default MimeMessage encoding.
    #[serde(default = "default_encoding")]
    pub default_encoding: String,

    /// Session properties.
    pub properties: Option<HashMap<String, String>>,

    /// SSL configuration.
    #[serde(default)]
    pub ssl: Ssl,
}

impl MailProperties {
    /// Returns the `host` field.
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// Returns the `port` field.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the `username` field.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns the `password` field.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns the `protocol` field.
    pub fn protocol(&self) -> &str {
        self.protocol.as_str()
    }

    /// Returns the `default_encoding` field.
    pub fn default_encoding(&self) -> &str {
        self.default_encoding.as_str()
    }

    /// Returns the `properties` field.
    pub fn properties(&self) -> Option<&HashMap<String, String>> {
        self.properties.as_ref()
    }

    /// Returns the `ssl` field.
    pub fn ssl(&self) -> &Ssl {
        &self.ssl
    }

    /// Sets the `host` field.
    pub fn set_host<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.host = value.into();
    }

    /// Sets the `port` field.
    pub fn set_port<V>(&mut self, value: V)
    where
        V: Into<u16>,
    {
        self.port = value.into();
    }

    /// Sets the `username` field.
    pub fn set_username<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.username = value.into();
    }

    /// Sets the `password` field.
    pub fn set_password<V>(&mut self, value: V)
    where
        V: Into<Option<String>>,
    {
        self.password = value.into();
    }

    /// Sets the `protocol` field.
    pub fn set_protocol(&mut self, value: String) {
        self.protocol = value;
    }

    /// Sets the `default_encoding` field.
    pub fn set_default_encoding(&mut self, value: String) {
        self.default_encoding = value;
    }

    /// Sets the `properties` field.
    pub fn set_properties<V>(&mut self, value: V)
    where
        V: Into<Option<HashMap<String, String>>>,
    {
        self.properties = value.into();
    }

    /// Sets the `ssl` field.
    pub fn set_ssl(&mut self, value: Ssl) {
        self.ssl = value;
    }
}

fn default_protocol() -> String {
    "smtp".to_string()
}

fn default_port() -> u16 {
    587
}

fn default_encoding() -> String {
    "UTF-8".to_string()
}
