use std::{any::TypeId, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

#[derive(Clone, Default)]
pub struct SimpleAuthentication {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AuthPrincipal>,
    authenticated: bool,
}

impl SimpleAuthentication {
    pub fn from_builder(builder: SimpleAuthenticationBuilder) -> Self {
        SimpleAuthentication {
            principal: builder.principal,
            credentials: builder.credentials,
            authorities: builder.authorities,
            details: builder.details,
            authenticated: builder.authenticated,
        }
    }

    pub fn builder() -> SimpleAuthenticationBuilder {
        SimpleAuthenticationBuilder::default()
    }
}

impl Authentication for SimpleAuthentication {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.details.as_ref()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    fn set_authenticated(&mut self, _is_authenticated: bool) -> Result<(), BoxError> {
        Err("Instead of calling this setter, please call builder to create a new instance".into())
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(SimpleAuthenticationBuilder::new(self)) as Box<dyn AuthenticationBuilder>
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for SimpleAuthentication {
    fn name(&self) -> &str {
        self.principal
            .as_ref()
            .and_then(|p| p.downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or_default()
    }
}

impl std::fmt::Display for SimpleAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SimpleAuthentication [Principal={}, Authenticated={}]",
            self.name(),
            self.is_authenticated()
        )
    }
}

#[derive(Clone, Default)]
pub struct SimpleAuthenticationBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    details: Option<AuthPrincipal>,
    authenticated: bool,
}

impl SimpleAuthenticationBuilder {
    pub fn new(authentication: &dyn Authentication) -> Self {
        Self {
            principal: authentication.principal().cloned(),
            credentials: authentication.credentials().cloned(),
            authorities: authentication.authorities().to_vec(),
            details: authentication.details().cloned(),
            authenticated: authentication.is_authenticated(),
        }
    }
}

impl AuthenticationBuilder for SimpleAuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        authorities(&mut self.authorities);
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.details = details;
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(SimpleAuthentication {
            principal: self.principal.take(),
            credentials: self.credentials.take(),
            authorities: std::mem::take(&mut self.authorities),
            details: self.details.take(),
            authenticated: self.authenticated,
        })
    }
}
