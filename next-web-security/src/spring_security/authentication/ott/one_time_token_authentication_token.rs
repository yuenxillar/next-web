use std::{
    any::TypeId,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{error::BoxError, AnyObject};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

/// An unauthenticated `Authentication` carrying a one-time token value.
///
/// This mirrors Spring Security's `OneTimeTokenAuthenticationToken` and is
/// produced by `OneTimeTokenAuthenticationConverter` and consumed by
/// `OneTimeTokenAuthenticationProvider`.
#[derive(Default)]
pub struct OneTimeTokenAuthenticationToken {
    principal: Option<AnyObject>,
    credentials: Option<AnyObject>,
    base: BaseAuthenticationToken,
}

impl OneTimeTokenAuthenticationToken {
    pub fn new(token_value: impl Into<String>) -> Self {
        Self::unauthenticated(token_value)
    }

    pub fn unauthenticated(token_value: impl Into<String>) -> Self {
        let mut token = Self {
            principal: None,
            credentials: Some(Arc::new(token_value.into())),
            base: BaseAuthenticationToken::new(None),
        };
        token
            .set_authenticated(false)
            .expect("Cannot set this token to trusted - use the authenticated constructor instead");
        token
    }

    pub fn authenticated(
        principal: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner.set_authenticated(true);

        Self {
            principal: Some(Arc::new(principal.into())),
            credentials: None,
            base: inner,
        }
    }

    pub fn token_value(&self) -> &str {
        self.credentials
            .as_ref()
            .and_then(|c| c.downcast_ref::<String>())
            .map(|s| s.as_str())
            .unwrap_or_default()
    }

    pub fn set_details_value(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }

    pub fn from_builder(builder: &mut OneTimeTokenAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Authentication for OneTimeTokenAuthenticationToken {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        if is_authenticated {
            return Err(
                "Cannot set this token to trusted - use the authenticated constructor instead".into(),
            );
        }
        self.base.set_authenticated(false);
        Ok(())
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(OneTimeTokenAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OneTimeTokenAuthenticationToken {
    fn name(&self) -> &str {
        self.base.name()
    }
}

impl Display for OneTimeTokenAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [Principal={:?}, Credentials=[PROTECTED], Authenticated={}, Authorities={:?}]",
            std::any::type_name::<Self>(),
            self.principal(),
            self.is_authenticated(),
            self.base
                .authorities()
                .iter()
                .map(|a| a.authority())
                .collect::<Vec<_>>()
        )
    }
}

impl Clone for OneTimeTokenAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            base: self.base.clone(),
        }
    }
}

/// A builder of `OneTimeTokenAuthenticationToken` instances.
pub struct OneTimeTokenAuthenticationTokenBuilder {
    principal: Option<AnyObject>,
    credentials: Option<AnyObject>,
    base: BaseAuthenticationBuilder,
}

impl OneTimeTokenAuthenticationTokenBuilder {
    fn with_token(token: &OneTimeTokenAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for OneTimeTokenAuthenticationTokenBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(authorities);
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.base.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.base.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(OneTimeTokenAuthenticationToken::from_builder(self))
    }
}

impl Deref for OneTimeTokenAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OneTimeTokenAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Deref for OneTimeTokenAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OneTimeTokenAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
