use crate::autoconfigure::context::application_context_properties::AppliationContextProperties;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppliationProperties {
    name: Option<String>,
    #[serde(default)]
    context: AppliationContextProperties,
}

impl AppliationProperties {
    pub fn new(name: Option<String>, context: AppliationContextProperties) -> Self {
        Self { name, context }
    }

    pub fn name(&self) -> &str {
        self.name.as_ref().map(|s| s.as_str()).unwrap_or_default()
    }

    pub fn context(&self) -> &AppliationContextProperties {
        &self.context
    }
}

impl Default for AppliationProperties {
    fn default() -> Self {
        Self {
            name: Some("application-dev".into()),
            context: Default::default(),
        }
    }
}
