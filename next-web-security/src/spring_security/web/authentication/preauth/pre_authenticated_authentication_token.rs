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

/// Authentication implementation for pre-authenticated authentication.
#[derive(Clone)]
pub struct PreAuthenticatedAuthenticationToken {
    principal: AuthPrincipal,
    credentials: Option<AuthPrincipal>,
    base: BaseAuthenticationToken,
}

impl PreAuthenticatedAuthenticationToken {
    /// Constructor used for an authentication request.
    ///
    /// [`Authentication::is_authenticated`] will return `false`.
    pub fn new(principal: AuthPrincipal, credentials: Option<AuthPrincipal>) -> Self {
        Self {
            principal,
            credentials,
            base: BaseAuthenticationToken::new(None),
        }
    }

    /// Constructor used for an authentication response.
    ///
    /// [`Authentication::is_authenticated`] will return `true`.
    pub fn with_authorities(
        principal: AuthPrincipal,
        credentials: Option<AuthPrincipal>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        Self {
            principal,
            credentials,
            base: {
                let mut base = BaseAuthenticationToken::new(Some(authorities));
                base.set_authenticated(true).ok();
                base
            },
        }
    }

    pub fn set_details(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }

    pub fn from_builder(builder: &mut PreAuthenticatedAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder
                .principal
                .take()
                .expect("Builder take principal is None"),
            credentials: builder.credentials.take(),
            base: BaseAuthenticationToken::from_builder(builder),
        }
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
        Some(&self.principal)
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
        crate::web::authentication::principal_name(self.principal.as_ref())
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

/// A builder of PreAuthenticatedAuthenticationToken instances
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

impl PreAuthenticatedAuthenticationTokenBuilder {
    fn with_token(token: &PreAuthenticatedAuthenticationToken) -> Self {
        Self {
            principal: Some(token.principal.clone()),
            credentials: token.credentials.clone(),
            base: BaseAuthenticationBuilder::with_token(token),
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
        Arc::new(PreAuthenticatedAuthenticationToken::from_builder(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::userdetails::User;

    /// Returns the token a pre-authenticated provider builds for a user details.
    fn token_of(user: User) -> PreAuthenticatedAuthenticationToken {
        PreAuthenticatedAuthenticationToken::with_authorities(
            Arc::new(user) as AuthPrincipal,
            None,
            Vec::new(),
        )
    }

    #[test]
    fn the_name_of_a_user_details_principal_is_the_user_name() {
        let token = token_of(
            User::with_username("Alice")
                .password(Some("secret".to_string()))
                .build(),
        );

        assert_eq!(
            token.name(),
            "Alice",
            "the name of an authentication is the user name, not the display of the user"
        );
    }

    #[test]
    fn the_name_of_another_principal_is_its_display_form() {
        let token = PreAuthenticatedAuthenticationToken::with_authorities(
            Arc::new("Alice".to_string()) as AuthPrincipal,
            None,
            Vec::new(),
        );

        assert_eq!(token.name(), "Alice");
    }
}
