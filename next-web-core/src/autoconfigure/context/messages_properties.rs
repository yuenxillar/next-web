#[derive(Debug, serde::Deserialize, Clone)]
pub struct MessagesProperties {
    local: Option<String>,
    base_name: Option<String>,

    /// The time in seconds to reload the messages properties file.
    /// Unused for now.
    #[serde(default)]
    reload_time: u32,
}

impl MessagesProperties {
    pub fn local(&self) -> Option<&str> {
        self.local.as_deref()
    }

    pub fn base_name(&self) -> Option<&str> {
        self.base_name.as_deref()
    }

    pub fn reload_time(&self) -> u32 {
        self.reload_time
    }
}

impl Default for MessagesProperties {
    fn default() -> Self {
        Self {
            local: Default::default(),
            base_name: Default::default(),
            reload_time: Default::default(),
        }
    }
}
