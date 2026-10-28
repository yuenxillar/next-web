use std::{
    any::TypeId,
    borrow::Cow,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

#[derive(Clone)]
pub struct TestingAuthenticationToken {
    principal: AuthPrincipal,
    credentials: Option<AuthPrincipal>,
    base: BaseAuthenticationToken,
}

impl TestingAuthenticationToken {
    pub fn new(principal: AuthPrincipal, credentials: Option<AuthPrincipal>) -> Self {
        Self::with_authorities(principal, credentials, Vec::new())
    }

    pub fn with_authorities(
        principal: AuthPrincipal,
        credentials: Option<AuthPrincipal>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut base = BaseAuthenticationToken::new(Some(authorities));
        let _ = base.set_authenticated(true);
        Self {
            principal,
            credentials,
            base,
        }
    }

    pub fn set_details_value(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }

    pub fn from_builder(builder: &mut TestingAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder
                .principal
                .take()
                .unwrap_or_else(|| Arc::new(String::new())),
            credentials: builder.credentials.take(),
            base: BaseAuthenticationToken::from_builder(builder),
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
        self.base.is_authenticated()
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn set_authenticated(
        &mut self,
        is_authenticated: bool,
    ) -> Result<(), next_web_core::error::BoxError> {
        self.base.set_authenticated(is_authenticated)
    }

    fn to_builder(&self) -> Box<dyn crate::core::AuthenticationBuilder> {
        Box::new(TestingAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for TestingAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        Cow::Owned(self.principal.to_string())
    }
}

impl Debug for TestingAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestingAuthenticationToken")
            .field("principal", &self.name())
            .field("authenticated", &self.is_authenticated())
            .finish()
    }
}

pub struct TestingAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    base: BaseAuthenticationBuilder,
}

impl TestingAuthenticationTokenBuilder {
    fn with_token(token: &TestingAuthenticationToken) -> Self {
        Self {
            principal: Some(token.principal.clone()),
            credentials: token.credentials.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for TestingAuthenticationTokenBuilder {
    fn authorities(&mut self, f: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(f);
    }
    fn credentials(&mut self, value: Option<AuthPrincipal>) {
        self.credentials = value;
    }
    fn details(&mut self, value: Option<AuthPrincipal>) {
        self.base.details(value);
    }
    fn principal(&mut self, value: Option<AuthPrincipal>) {
        self.principal = value;
    }
    fn authenticated(&mut self, value: bool) {
        self.base.authenticated(value);
    }
    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(TestingAuthenticationToken::from_builder(self))
    }
}

impl Deref for TestingAuthenticationToken {
    type Target = BaseAuthenticationToken;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for TestingAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
impl Deref for TestingAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for TestingAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
