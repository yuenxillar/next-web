#[derive(Clone, Debug, serde::Deserialize)]
pub struct MQTTClientProperties {
    id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    topics: Option<Vec<String>>,
    keep_alive: Option<u64>,
}

impl MQTTClientProperties {
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

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

    pub fn topics(&self) -> Vec<String> {
        if let Some(topics) = self.topics.as_ref() {
            return topics.clone();
        }
        vec![]
    }

    pub fn keep_alive(&self) -> Option<u64> {
        self.keep_alive
    }
}

impl Default for MQTTClientProperties {
    fn default() -> Self {
        Self {
            id: Some(String::from("next-web-mqtt")),
            host: Some(String::from("localhost")),
            port: Some(1883),
            keep_alive: Some(5),
            username: Some(String::new()),
            password: Some(String::new()),
            topics: Some(vec![String::from("test/#")]),
        }
    }
}
