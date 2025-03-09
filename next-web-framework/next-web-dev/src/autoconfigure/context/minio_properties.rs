#[derive(Debug, Clone, serde::Deserialize)]
pub struct MinioProperties {
    endpoint: String,
    access_key: String,
    secret_key: String,
    buckets: Option<Vec<String>>,
}

impl MinioProperties {
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

impl Default for MinioProperties {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:9000".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            buckets: None,
        }
    }
}
