use next_web_macros::Properties;
use rudi_dev::Singleton;

/// Properties for Minio client.
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.data.minio")]
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MinioClientProperties {
    /// Minio endpoint.
    endpoint: String,
    /// Minio access key.
    access_key: String,
    /// Minio secret key.
    secret_key: String,
    /// List of buckets to create.
    buckets: Option<Vec<String>>,
}

impl MinioClientProperties {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn access_key(&self) -> &str {
        &self.access_key
    }

    pub fn secret_key(&self) -> &str {
        &self.secret_key
    }

    pub fn buckets(&self) -> Option<&Vec<String>> {
        self.buckets.as_ref()
    }
}

impl Default for MinioClientProperties {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:9000".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            buckets: None,
        }
    }
}
