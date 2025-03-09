#[derive(Debug, Clone, serde::Deserialize)]
pub struct MessageSourceProperties {
    basename: Option<String>,
    encoding: Option<String>,
    fallback_to_system_locale: Option<bool>,
}

impl MessageSourceProperties {
    pub fn basename(&self) -> Option<&str> {
        self.basename.as_deref()
    }
    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_deref()
    }

    pub fn fallback_to_system_locale(&self) -> Option<bool> {
        self.fallback_to_system_locale
    }
}
