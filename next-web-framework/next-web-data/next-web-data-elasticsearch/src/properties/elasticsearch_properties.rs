use next_web_macros::Properties;
use rudi_dev::Singleton;

/// Properties for Elasticsearch client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.elasticsearch")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ElasticsearchClientProperties {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
}

impl ElasticsearchClientProperties {
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }
}

impl Default for ElasticsearchClientProperties {
    fn default() -> Self {
        Self {
            host: Some("localhost".into()),
            port: Some(9200),
            username: None,
            password: None,
        }
    }
}
