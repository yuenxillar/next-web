//! The OAuth 2.0 user.

use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct OAuth2User {
    name: String,
    attributes: HashMap<String, String>,
}

impl OAuth2User {
    pub fn new(name: impl Into<String>, attributes: HashMap<String, String>) -> Self {
        Self {
            name: name.into(),
            attributes,
        }
    }
}

impl OAuth2User {
    /// Returns the name of the user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the attributes of the user.
    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}
