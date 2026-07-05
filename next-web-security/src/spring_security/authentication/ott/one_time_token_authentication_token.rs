use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::core::{granted_authority::GrantedAuthority, Authentication};

#[derive(Clone, Default)]
pub struct OneTimeTokenAuthenticationToken {
    principal: Option<String>,
    token_value: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AnyValue>,
    authenticated: bool,
}

impl OneTimeTokenAuthenticationToken {
    pub fn new(token_value: impl Into<String>) -> Self {
        Self {
            principal: None,
            token_value: token_value.into(),
            authorities: Vec::new(),
            details: None,
            authenticated: false,
        }
    }

    pub fn unauthenticated(token_value: impl Into<String>) -> Self {
        Self::new(token_value)
    }

    pub fn authenticated(
        principal: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal: Some(principal.into()),
            token_value: String::new(),
            authorities,
            details: None,
            authenticated: true,
        }
    }

    pub fn token_value(&self) -> &str {
        &self.token_value
    }

    pub fn set_details_value(&mut self, details: Option<AnyValue>) {
        self.details = details;
    }
}

impl Authentication for OneTimeTokenAuthenticationToken {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn authentication_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn get_credentials(&self) -> Option<String> {
        Some(self.token_value.clone())
    }

    fn get_details_ref(&self) -> Option<&AnyValue> {
        self.details.as_ref()
    }

    fn get_principal(&self) -> Option<String> {
        self.principal.clone()
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn authorities(&self) -> Vec<String> {
        self.authorities
            .iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect()
    }
}
