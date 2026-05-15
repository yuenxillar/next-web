use std::sync::Arc;

use crate::core::{
    granted_authority::GrantedAuthority, simple_granted_authority::SimpleGrantedAuthority,
};

#[derive(Clone, Default)]
pub struct UserAttribute {
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    password: Option<String>,
    enabled: bool,
}

impl UserAttribute {
    pub fn new() -> Self {
        Self {
            authorities: Vec::new(),
            password: None,
            enabled: true,
        }
    }

    pub fn add_authority(&mut self, new_authority: Arc<dyn GrantedAuthority>) {
        self.authorities.push(new_authority);
    }

    pub fn authorities(&self) -> Vec<Arc<dyn GrantedAuthority>> {
        self.authorities.clone()
    }

    pub fn set_authorities(&mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) {
        self.authorities = authorities;
    }

    pub fn set_authorities_as_string(
        &mut self,
        authorities_as_strings: impl IntoIterator<Item = impl Into<String>>,
    ) {
        self.authorities = Vec::new();
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
