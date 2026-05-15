#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct GrantedAuthorityDefaults {
    role_prefix: String,
}

impl GrantedAuthorityDefaults {
    pub fn new(role_prefix: impl Into<String>) -> Self {
        Self {
            role_prefix: role_prefix.into(),
        }
    }

    pub fn role_prefix(&self) -> &str {
        &self.role_prefix
    }
}

impl Default for GrantedAuthorityDefaults {
    fn default() -> Self {
        Self::new("ROLE_")
    }
}

#[cfg(test)]
mod tests {
    use super::GrantedAuthorityDefaults;

    #[test]
    fn defaults_exposes_configured_role_prefix() {
        let defaults = GrantedAuthorityDefaults::new("PREFIX_");

        assert_eq!(defaults.role_prefix(), "PREFIX_");
    }
}
