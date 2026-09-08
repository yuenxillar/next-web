use std::sync::Arc;

use crate::{
    core::{Authentication, GrantedAuthority},
    web::authentication::AuthPrincipal,
};

#[derive(Clone, Default)]
pub struct TestingAuthenticationToken {
    principal: AuthPrincipal,
    credentials: Option<AuthPrincipal>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl TestingAuthenticationToken {
    pub fn new(principal: AuthPrincipal, credentials: Option<AuthPrincipal>) -> Self {
        Self {
            principal,
            credentials,
            authorities: Vec::new(),
        }
    }

    pub fn with_authorities(
        principal: AuthPrincipal,
        credentials: Option<AuthPrincipal>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal,
            credentials,
            authorities,
        }
    }
}

impl Authentication for TestingAuthenticationToken {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials.as_ref()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        Some(&self.principal)
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

    fn details(&self) -> Option<&AuthPrincipal> {
        todo!()
    }

    fn of(&self) -> std::any::TypeId {
        todo!()
    }

    fn set_authenticated(
        &mut self,
        is_authenticated: bool,
    ) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }

    fn to_builder(&self) -> Box<dyn crate::core::AuthenticationBuilder> {
        todo!()
    }
}
