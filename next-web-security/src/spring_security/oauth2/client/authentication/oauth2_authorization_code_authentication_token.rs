use std::{any::TypeId, borrow::Cow, fmt, sync::Arc};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    oauth2::{
        ClientRegistration, OAuth2AccessToken, OAuth2AuthorizationExchange, OAuth2AuthorizedClient,
    },
    web::authentication::AuthPrincipal,
};

use super::oauth2_authorization_code_request_data::OAuth2AuthorizationCodeRequestData;

/// An `Authentication` representing the result of the OAuth 2.0 Authorization
/// Code Grant. While unauthenticated it carries the authorization request/
/// response exchange; once authenticated it carries the issued access token
/// and the persisted `OAuth2AuthorizedClient`.
pub struct OAuth2AuthorizationCodeAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    principal_name: String,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    access_token: Option<OAuth2AccessToken>,
    authorized_client: Option<OAuth2AuthorizedClient>,
    base: BaseAuthenticationToken,
}

impl OAuth2AuthorizationCodeAuthenticationToken {
    pub fn unauthenticated(
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
    ) -> Self {
        let credentials = Arc::new(OAuth2AuthorizationCodeRequestData {
            client_registration: client_registration.clone(),
            authorization_exchange: authorization_exchange.clone(),
        }) as AuthPrincipal;
        let mut token = Self {
            principal: None,
            credentials: Some(credentials),
            principal_name: String::new(),
            client_registration,
            authorization_exchange,
            access_token: None,
            authorized_client: None,
            base: BaseAuthenticationToken::new(None),
        };
        token
            .set_authenticated(false)
            .expect("OAuth2AuthorizationCodeAuthenticationToken cannot be set trusted directly");
        token
    }

    pub fn authenticated(
        principal_name: impl Into<String>,
        client_registration: ClientRegistration,
        authorization_exchange: OAuth2AuthorizationExchange,
        access_token: OAuth2AccessToken,
        authorized_client: OAuth2AuthorizedClient,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner
            .set_authenticated(true)
            .expect("authenticated OAuth2 code token should accept trusted state");
        let principal_name = principal_name.into();
        let credentials = Arc::new(access_token.clone()) as AuthPrincipal;
        let principal = Arc::new(authorized_client.clone()) as AuthPrincipal;
        Self {
            principal: Some(principal),
            credentials: Some(credentials),
            principal_name,
            client_registration,
            authorization_exchange,
            access_token: Some(access_token),
            authorized_client: Some(authorized_client),
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

    pub fn authorized_client(&self) -> Option<&OAuth2AuthorizedClient> {
        self.authorized_client.as_ref()
    }

    fn from_builder(builder: &mut OAuth2AuthorizationCodeAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            principal_name: builder.principal_name.clone(),
            client_registration: builder.client_registration.clone(),
            authorization_exchange: builder.authorization_exchange.clone(),
            access_token: builder.access_token.clone(),
            authorized_client: builder.authorized_client.clone(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Clone for OAuth2AuthorizationCodeAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            principal_name: self.principal_name.clone(),
            client_registration: self.client_registration.clone(),
            authorization_exchange: self.authorization_exchange.clone(),
            access_token: self.access_token.clone(),
            authorized_client: self.authorized_client.clone(),
            base: self.base.clone(),
        }
    }
}

impl Authentication for OAuth2AuthorizationCodeAuthenticationToken {
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
        Box::new(OAuth2AuthorizationCodeAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OAuth2AuthorizationCodeAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        if self.principal_name.is_empty() {
            Cow::Borrowed(self.client_registration.registration_id())
        } else {
            Cow::Borrowed(&self.principal_name)
        }
    }
}

impl fmt::Debug for OAuth2AuthorizationCodeAuthenticationToken {
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

pub struct OAuth2AuthorizationCodeAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    principal_name: String,
    client_registration: ClientRegistration,
    authorization_exchange: OAuth2AuthorizationExchange,
    access_token: Option<OAuth2AccessToken>,
    authorized_client: Option<OAuth2AuthorizedClient>,
    base: BaseAuthenticationBuilder,
}

impl OAuth2AuthorizationCodeAuthenticationTokenBuilder {
    fn with_token(token: &OAuth2AuthorizationCodeAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),
            principal_name: token.principal_name.clone(),
            client_registration: token.client_registration.clone(),
            authorization_exchange: token.authorization_exchange.clone(),
            access_token: token.access_token.clone(),
            authorized_client: token.authorized_client.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for OAuth2AuthorizationCodeAuthenticationTokenBuilder {
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
        Arc::new(OAuth2AuthorizationCodeAuthenticationToken::from_builder(
            self,
        ))
    }
}

impl std::ops::Deref for OAuth2AuthorizationCodeAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for OAuth2AuthorizationCodeAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl std::ops::Deref for OAuth2AuthorizationCodeAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for OAuth2AuthorizationCodeAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
