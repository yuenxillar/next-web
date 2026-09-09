use std::{
    any::TypeId,
    borrow::Cow,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};
use next_web_core::error::BoxError;

#[derive(Clone, Default)]
pub struct PreAuthenticatedAuthenticationToken {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    base: BaseAuthenticationToken,
}

impl PreAuthenticatedAuthenticationToken {
    pub fn unauthenticated(principal: Option<String>, credentials: Option<String>) -> Self {
        Self {
            principal: principal.map(|v| Arc::new(v) as AuthPrincipal),
            credentials: credentials.map(|v| Arc::new(v) as AuthPrincipal),
            base: BaseAuthenticationToken::new(None),
        }
    }

    pub fn authenticated(
        principal: impl Into<String>,
        credentials: Option<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal: Some(Arc::new(principal.into())),
            credentials: credentials.map(|v| Arc::new(v) as AuthPrincipal),
            base: {
                let mut b = BaseAuthenticationToken::new(Some(authorities));
                let _ = b.set_authenticated(true);
                b
            },
        }
    }

    pub fn set_details(&mut self, details: Option<String>) {
        self.base
            .set_details(details.map(|v| Arc::new(v) as AuthPrincipal));
    }

    pub fn set_details_value(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }
}

impl Authentication for PreAuthenticatedAuthenticationToken {
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

    fn set_authenticated(&mut self, v: bool) -> Result<(), BoxError> {
        self.base.set_authenticated(v)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(PreAuthenticatedAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for PreAuthenticatedAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        Cow::Owned(
            self.principal
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
    }
}

impl Display for PreAuthenticatedAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PreAuthenticatedAuthenticationToken [Principal={}, Authenticated={}]",
            self.name(),
            self.is_authenticated()
        )
    }
}

impl Debug for PreAuthenticatedAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreAuthenticatedAuthenticationToken")
            .field("principal", &self.name())
            .field("authenticated", &self.is_authenticated())
            .finish()
    }
}

pub struct PreAuthenticatedAuthenticationTokenBuilder {
    principal: Option<AuthPrincipal>,
    credentials: Option<AuthPrincipal>,
    base: BaseAuthenticationBuilder,
}

impl Deref for PreAuthenticatedAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}
impl DerefMut for PreAuthenticatedAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl PreAuthenticatedAuthenticationToken {
    pub fn get_name(&self) -> String {
        self.name().into_owned()
    }

    pub fn get_credentials(&self) -> Option<String> {
        self.credentials.as_ref().map(ToString::to_string)
    }

    pub fn get_details_ref(&self) -> Option<&AuthPrincipal> {
        self.details()
    }

    pub fn get_details_value(&self) -> Option<AuthPrincipal> {
        self.details().cloned()
    }
}

impl PreAuthenticatedAuthenticationTokenBuilder {
    fn with_token(t: &PreAuthenticatedAuthenticationToken) -> Self {
        Self {
            principal: t.principal.clone(),
            credentials: t.credentials.clone(),
            base: BaseAuthenticationBuilder::with_token(t),
        }
    }
}

impl Deref for PreAuthenticatedAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for PreAuthenticatedAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl AuthenticationBuilder for PreAuthenticatedAuthenticationTokenBuilder {
    fn authorities(&mut self, f: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(f);
    }

    fn credentials(&mut self, v: Option<AuthPrincipal>) {
        self.credentials = v;
    }

    fn details(&mut self, v: Option<AuthPrincipal>) {
        self.base.details(v);
    }

    fn principal(&mut self, v: Option<AuthPrincipal>) {
        self.principal = v;
    }

    fn authenticated(&mut self, v: bool) {
        self.base.authenticated(v);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(PreAuthenticatedAuthenticationToken {
            principal: self.principal.take(),
            credentials: self.credentials.take(),
            base: BaseAuthenticationToken::from_builder(self),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{authority::AuthorityUtils, Authentication};

    use super::PreAuthenticatedAuthenticationToken;

    #[test]
    fn pre_authenticated_authentication_token_supports_authenticated_constructor() {
        let token = PreAuthenticatedAuthenticationToken::authenticated(
            "alice",
            Some(String::from("external-credential")),
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        assert!(token.is_authenticated());
        assert_eq!(token.get_name(), "alice");
    }
}
