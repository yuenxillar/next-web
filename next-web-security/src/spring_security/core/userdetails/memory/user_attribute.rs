use std::sync::Arc;

use crate::core::{authority::SimpleGrantedAuthority, GrantedAuthority};

/// Used by security.provisioning.InMemoryUserDetailsManager to temporarily store the
/// attributes associated with a user.
#[derive(Clone)]
pub struct UserAttribute {
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    password: Option<String>,
    enabled: bool,
}

impl UserAttribute {
    pub fn add_authority(&mut self, new_authority: Arc<dyn GrantedAuthority>) {
        self.authorities.push(new_authority);
    }

    pub fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }

    /// Set all authorities for this user.
    pub fn set_authorities(&mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) {
        self.authorities = authorities;
    }

    /// Set all authorities for this user from String values. It will create the necessary GrantedAuthority objects
    pub fn set_authorities_as_string<I, V>(&mut self, authorities_as_strings: I)
    where
        I: IntoIterator<Item = V>,
        V: Into<String>,
    {
        self.set_authorities(Vec::new());
        for authority in authorities_as_strings {
            self.add_authority(Arc::new(SimpleGrantedAuthority::new(authority)));
        }
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_valid(&self) -> bool {
        self.password.is_some() && !self.authorities.is_empty()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_password(&mut self, password: impl Into<String>) {
        self.password = Some(password.into());
    }
}

impl Default for UserAttribute {
    fn default() -> Self {
        Self {
            authorities: Vec::new(),
            password: None,
            enabled: true,
        }
    }
}
