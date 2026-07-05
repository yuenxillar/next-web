use std::sync::Arc;

use next_web_core::anys::any_value::AnyValue;

use crate::core::{granted_authority::GrantedAuthority, Authentication};

#[derive(Clone)]
pub struct OneTimeTokenAuthentication {
    principal: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AnyValue>,
}

impl OneTimeTokenAuthentication {
    pub fn new(principal: impl Into<String>, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        let principal = principal.into();
        assert!(!principal.trim().is_empty(), "principal cannot be empty");
        Self {
            principal,
            authorities,
            details: None,
        }
    }

    pub fn set_details_value(&mut self, details: Option<AnyValue>) {
        self.details = details;
    }
}

impl Authentication for OneTimeTokenAuthentication {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn authentication_type(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn get_details_ref(&self) -> Option<&AnyValue> {
        self.details.as_ref()
    }

    fn get_principal(&self) -> Option<String> {
        Some(self.principal.clone())
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn authorities(&self) -> Vec<String> {
        self.authorities
            .iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect()
    }
}
