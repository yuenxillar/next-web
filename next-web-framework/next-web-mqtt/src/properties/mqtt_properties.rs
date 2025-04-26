use rudi_dev::{Properties, Singleton};

#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.mqtt")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct MQTTClientProperties {
    client_id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    topics: Option<Vec<String>>,
    keep_alive: Option<u64>,
    clean_session: Option<bool>
}

impl MQTTClientProperties {
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
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

    pub fn clean_session(&self) -> Option<bool> {
        self.clean_session
    }
}