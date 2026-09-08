use std::{
    any::TypeId,
    borrow::Cow,
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{
        Authentication, AuthenticationBuilder, AuthenticationError, GrantedAuthority, Principal,
    },
    web::{
        authentication::{AuthPrincipal, AuthenticationConverter},
        util::matcher::RequestMatcher,
    },
};

/// Resolves a bearer token from an `HttpRequest`.
pub trait BearerTokenResolver: Send + Sync {
    fn resolve(&self, request: &dyn HttpRequest) -> Option<String>;
}

/// The default `BearerTokenResolver`, which reads the `Authorization` header and
/// strips the `Bearer ` prefix.
#[derive(Clone, Debug, Default)]
pub struct DefaultBearerTokenResolver;

impl BearerTokenResolver for DefaultBearerTokenResolver {
    fn resolve(&self, request: &dyn HttpRequest) -> Option<String> {
        let authorization = request.header("Authorization")?;
        authorization
            .strip_prefix("Bearer ")
            .or_else(|| authorization.strip_prefix("Bearer\t"))
            .map(|token| token.trim().to_string())
    }
}

/// A `RequestMatcher` that matches requests carrying a bearer token.
#[derive(Clone, Debug, Default)]
pub struct BearerTokenRequestMatcher;

impl RequestMatcher for BearerTokenRequestMatcher {
    fn matches(&self, request: &dyn HttpRequest) -> bool {
        request
            .header("Authorization")
            .map(|header| header.starts_with("Bearer "))
            .unwrap_or(false)
    }
}

/// An `Authentication` representing a bearer token presented to a resource
/// server. While unauthenticated it carries the raw bearer token string; once
/// authenticated it carries the resolved principal and authorities.
pub struct BearerTokenAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    token: Option<String>,
    base: BaseAuthenticationToken,
}

impl BearerTokenAuthenticationToken {
    pub fn unauthenticated(token: impl Into<String>) -> Self {
        let token = token.into();
        let credentials = Arc::new(token.clone()) as AuthPrincipal;
        let mut authentication = Self {
            principal: None,
            credentials: Some(credentials),
            token: Some(token),
            base: BaseAuthenticationToken::new(None),
        };
        authentication
            .set_authenticated(false)
            .expect("BearerTokenAuthenticationToken cannot be set trusted directly");
        authentication
    }

    pub fn authenticated(
        principal: AuthPrincipal,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
        token: impl Into<String>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner
            .set_authenticated(true)
            .expect("authenticated bearer token should accept trusted state");
        Self {
            principal: Some(principal),
            credentials: None,
            token: Some(token.into()),
            base: inner,
        }
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    fn from_builder(builder: &mut BearerTokenAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            token: builder.token.clone(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Clone for BearerTokenAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            token: self.token.clone(),
            base: self.base.clone(),
        }
    }
}

impl Authentication for BearerTokenAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

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

    fn set_authenticated(
        &mut self,
        is_authenticated: bool,
    ) -> Result<(), next_web_core::error::BoxError> {
        if is_authenticated {
            return Err("Cannot set this token to trusted - use authenticated()".into());
        }
        self.base.set_authenticated(false)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(BearerTokenAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for BearerTokenAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        self.principal
            .as_ref()
            .map(|principal| Cow::Owned(principal.to_string()))
            .unwrap_or_default()
    }
}

impl fmt::Debug for BearerTokenAuthenticationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [Principal={}, Authenticated={}, Token={}]",
            std::any::type_name::<Self>(),
            self.name(),
            self.is_authenticated(),
            self.token.as_deref().unwrap_or("")
        )
    }
}

pub struct BearerTokenAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    token: Option<String>,
    base: BaseAuthenticationBuilder,
}

impl BearerTokenAuthenticationTokenBuilder {
    fn with_token(token: &BearerTokenAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            token: token.token.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for BearerTokenAuthenticationTokenBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(authorities);
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.base.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.base.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(BearerTokenAuthenticationToken::from_builder(self))
    }
}

impl Deref for BearerTokenAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BearerTokenAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Deref for BearerTokenAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BearerTokenAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Converts a bearer token request into an unauthenticated
/// `BearerTokenAuthenticationToken` for downstream authentication.
#[derive(Clone)]
pub struct BearerTokenAuthenticationConverter {
    bearer_token_resolver: Arc<dyn BearerTokenResolver>,
}

impl BearerTokenAuthenticationConverter {
    pub fn new(bearer_token_resolver: Arc<dyn BearerTokenResolver>) -> Self {
        Self {
            bearer_token_resolver,
        }
    }

    pub fn with_default_resolver() -> Self {
        Self::new(Arc::new(DefaultBearerTokenResolver::default()))
    }
}

impl AuthenticationConverter for BearerTokenAuthenticationConverter {
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        let token = match self.bearer_token_resolver.resolve(request) {
            Some(token) => token,
            None => return Ok(None),
        };
        Ok(Some(Box::new(
            BearerTokenAuthenticationToken::unauthenticated(token),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
