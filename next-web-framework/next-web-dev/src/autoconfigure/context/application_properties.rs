#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppliationProperties {
    name: Option<String>,
}

impl AppliationProperties {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref().map(|s| s.as_str()).unwrap_or_default()
    }
}
