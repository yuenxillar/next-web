use std::{
    any::TypeId,
    borrow::Cow,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    oauth2::{ClientRegistration, OAuth2AccessToken, OAuth2AuthorizationExchange},
    web::authentication::AuthPrincipal,
};

pub struct OAuth2LoginAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    access_token: Option<OAuth2AccessToken>,
    cleared: AtomicBool,
    base: BaseAuthenticationToken,
}

impl OAuth2LoginAuthenticationToken {
    pub fn unauthenticated(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        let credentials = Arc::new(authorization_exchange.clone()) as AuthPrincipal;
        let mut token = Self {
            principal: None,
            credentials: Some(credentials),
            client_registration,
            authorization_exchange,
            access_token: None,
            cleared: AtomicBool::new(false),
            base: BaseAuthenticationToken::new(None),
        };
        token
            .set_authenticated(false)
            .expect("OAuth2LoginAuthenticationToken cannot be set trusted directly");
        token
    }

    pub fn authenticated(
        principal: AuthPrincipal,
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
        access_token: OAuth2AccessToken,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner
            .set_authenticated(true)
            .expect("authenticated OAuth2 token should accept trusted state");
        Self {
            principal: Some(principal),
            credentials: None,
            client_registration,
            authorization_exchange,
            access_token: Some(access_token),
            cleared: AtomicBool::new(false),
            base: inner,
        }
    }

    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    pub fn authorization_exchange(&self) -> &OAuth2AuthorizationExchange {
        &self.authorization_exchange
    }

    pub fn access_token(&self) -> Option<&OAuth2AccessToken> {
        self.access_token.as_ref()
    }

    fn from_builder(builder: &mut OAuth2LoginAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            client_registration: builder.client_registration.clone(),
            authorization_exchange: builder.authorization_exchange.clone(),
            access_token: builder.access_token.clone(),
            cleared: AtomicBool::new(false),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Clone for OAuth2LoginAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            client_registration: self.client_registration.clone(),
            authorization_exchange: self.authorization_exchange.clone(),
            access_token: self.access_token.clone(),
            cleared: AtomicBool::new(self.cleared.load(Ordering::Acquire)),
            base: self.base.clone(),
        }
    }
}

impl Authentication for OAuth2LoginAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        if self.cleared.load(Ordering::Acquire) {
            return None;
        }
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
        Box::new(OAuth2LoginAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OAuth2LoginAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        self.principal
            .as_ref()
            .map(|principal| Cow::Owned(principal.to_string()))
            .unwrap_or_else(|| Cow::Borrowed(self.client_registration.registration_id()))
    }
}

impl fmt::Debug for OAuth2LoginAuthenticationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [Principal={}, Authenticated={}, ClientRegistration={}]",
            std::any::type_name::<Self>(),
            self.name(),
            self.is_authenticated(),
            self.client_registration.registration_id()
        )
    }
}

pub struct OAuth2LoginAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    access_token: Option<OAuth2AccessToken>,
    base: BaseAuthenticationBuilder,
}

impl OAuth2LoginAuthenticationTokenBuilder {
    fn with_token(token: &OAuth2LoginAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            client_registration: token.client_registration.clone(),
            authorization_exchange: token.authorization_exchange.clone(),
            access_token: token.access_token.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for OAuth2LoginAuthenticationTokenBuilder {
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
        Arc::new(OAuth2LoginAuthenticationToken::from_builder(self))
    }
}

impl std::ops::Deref for OAuth2LoginAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for OAuth2LoginAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl std::ops::Deref for OAuth2LoginAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for OAuth2LoginAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
