use next_web_macros::configuration_properties;

/// MQTT client configuration properties.
///
/// The properties are bound from the `next.mqtt` prefix of the application
/// configuration and registered under the `mQTTClientProperties` singleton
/// name, which is what the auto-configuration autowires them by.
#[configuration_properties(prefix = "next.mqtt", dynamic)]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct MQTTClientProperties {
    /// Client identifier (optional).
    ///
    /// An empty identifier leaves the choice to the broker, which is only
    /// allowed while `clean_session` is `true`.
    client_id: Option<String>,

    /// Broker URL (optional, defaults to `tcp://127.0.0.1:1883`).
    ///
    /// The scheme selects the transport and the URL carries the address:
    ///
    /// | Scheme | Transport | Default port |
    /// | --- | --- | --- |
    /// | `tcp`, `mqtt` | Plain TCP | `1883` |
    /// | `ssl`, `mqtts` | TCP over TLS | `8883` |
    /// | `ws` | WebSocket | `80` |
    /// | `wss` | WebSocket over TLS | `443` |
    ///
    /// The `ws` and `wss` schemes require the `ws` feature of this crate. When
    /// the URL has no port the default port of its scheme is used, and a
    /// WebSocket URL may end with a path, for example
    /// `wss://broker.example.com:8000/mqtt`.
    url: Option<String>,

    /// Authentication username (optional).
    username: Option<String>,

    /// Authentication password (optional).
    password: Option<String>,

    /// Topics to subscribe to after connecting (optional).
    ///
    /// Each entry is a topic filter with an optional QoS level:
    ///
    /// ```yaml
    /// topics:
    ///   - topic: "sensors/+/temperature"
    ///     qos: 1
    ///   - topic: "alerts/#"
    /// ```
    ///
    /// The QoS of an entry defaults to `AtLeastOnce` when it is omitted.
    topics: Option<Vec<Topic>>,

    /// Keep alive interval in seconds (optional, defaults to `60`).
    keep_alive: Option<u64>,

    /// Connection timeout in seconds (optional, defaults to `10`).
    connect_timeout: Option<u64>,

    /// Clean session flag (optional, defaults to `true`).
    clean_session: Option<bool>,

    /// Capacity of the bounded async channel between the client and the event
    /// loop (optional, defaults to `2048`).
    ///
    /// The capacity has to cover the peak backlog of the message rate:
    /// `(message generation rate - message processing rate) x buffer time`.
    cap: Option<usize>,

    /// Maximum number of connection retries (optional, defaults to `5`).
    ///
    /// When the limit is reached the application stops retrying and shuts the
    /// event loop down.
    max_retries: Option<u16>,
}

impl MQTTClientProperties {
    /// Returns the configured client identifier.
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Returns the configured broker URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Returns the configured authentication username.
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns the configured authentication password.
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns the configured subscription topics.
    ///
    /// An empty vector is returned when no topic is configured.
    pub fn topics(&self) -> Vec<Topic> {
        self.topics.clone().unwrap_or_default()
    }

    /// Returns the keep alive interval in seconds.
    pub fn keep_alive(&self) -> Option<u64> {
        self.keep_alive
    }

    /// Returns the connection timeout in seconds.
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// Returns the clean session flag.
    pub fn clean_session(&self) -> Option<bool> {
        self.clean_session
    }

    /// Returns the capacity of the bounded async channel.
    pub fn cap(&self) -> Option<usize> {
        self.cap
    }

    /// Returns the maximum number of connection retries.
    pub fn max_retries(&self) -> Option<u16> {
        self.max_retries
    }

    /// Sets the client identifier.
    pub fn set_client_id(&mut self, client_id: impl ToString) {
        self.client_id = Some(client_id.to_string());
    }

    /// Sets the broker URL.
    pub fn set_url(&mut self, url: impl ToString) {
        self.url = Some(url.to_string());
    }

    /// Sets the authentication username.
    pub fn set_username(&mut self, username: impl ToString) {
        self.username = Some(username.to_string());
    }

    /// Sets the authentication password.
    pub fn set_password(&mut self, password: impl ToString) {
        self.password = Some(password.to_string());
    }

    /// Sets the subscription topics.
    pub fn set_topics(&mut self, topics: impl IntoIterator<Item = Topic>) {
        self.topics = Some(topics.into_iter().collect());
    }

    /// Sets the keep alive interval in seconds.
    pub fn set_keep_alive(&mut self, keep_alive: u64) {
        self.keep_alive = Some(keep_alive);
    }

    /// Sets the connection timeout in seconds.
    pub fn set_connect_timeout(&mut self, connect_timeout: u64) {
        self.connect_timeout = Some(connect_timeout);
    }

    /// Sets the clean session flag.
    pub fn set_clean_session(&mut self, clean_session: bool) {
        self.clean_session = Some(clean_session);
    }

    /// Sets the capacity of the bounded async channel.
    pub fn set_cap(&mut self, cap: usize) {
        self.cap = Some(cap);
    }

    /// Sets the maximum number of connection retries.
    pub fn set_max_retries(&mut self, max_retries: u16) {
        self.max_retries = Some(max_retries);
    }
}

/// A topic filter to subscribe to after connecting.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct Topic {
    /// Broker topic filter, for example `sensors/+/temperature` or `alerts/#`.
    pub topic: String,
    /// Requested QoS level, which defaults to `AtLeastOnce` when omitted.
    pub qos: Option<u8>,
}
