use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliationContextProperties {
    allow_override: bool,
}

impl AppliationContextProperties {
    pub fn new(allow_override: bool) -> Self {
        Self { allow_override }
    }

    pub fn allow_override(&self) -> bool {
        self.allow_override
    }
}

impl Default for AppliationContextProperties {
    fn default() -> Self {
        Self {
            allow_override: Default::default(),
        }
    }
}
