use std::{
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use next_web_core::{error::BoxError, AnyObject};

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{
        credentials_container::CredentialsContainer, granted_authority::GrantedAuthority,
        Authentication, AuthenticationBuilder, Principal,
    },
    web::authentication::AuthPrincipal,
};

/// An `Authentication` implementation that is designed for simple presentation of a
/// username and password.
///
/// The `principal` and `credentials` should be set with an `Object` that provides the
/// respective property via its `to_string()` method. The simplest such `Object` to use is
/// `String`.
#[derive(Default)]
pub struct UsernamePasswordAuthenticationToken {
    principal: Option<AnyObject>,
    credentials: Option<AnyObject>,

    cleared: AtomicBool,
    inner: BaseAuthenticationToken,
}

impl UsernamePasswordAuthenticationToken {
    /// This constructor should only be used by `AuthenticationManager` or
    /// `AuthenticationProvider` implementations that are satisfied with
    /// producing a trusted (i.e. `is_authenticated()` = `true`)
    /// authentication token.
    ///
    /// # Arguments
    /// * `principal` - the principal (typically a `UserDetails` instance)
    /// * `credentials` - the credentials (typically a password)
    /// * `authorities` - the granted authorities
    pub fn authenticated(
        principal: AuthPrincipal,
        credentials: Option<AuthPrincipal>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner.set_authenticated(true); // must use super, as we override

        Self {
            principal: Some(principal),
            credentials: credentials,
            cleared: AtomicBool::new(false),

            inner,
        }
    }

    /// This factory method can be safely used by any code that wishes to create a
    /// unauthenticated `UsernamePasswordAuthenticationToken`.
    ///
    /// # Arguments
    /// * `principal` - the principal
    /// * `credentials` - the credentials
    ///
    /// # Returns
    /// `UsernamePasswordAuthenticationToken` with `is_authenticated()` returning `false`
    pub fn unauthenticated(
        principal: Option<AuthPrincipal>,
        credentials: Option<AuthPrincipal>,
    ) -> Self {
        let mut token = Self {
            principal: principal.into(),
            credentials: credentials.into(),
            cleared: AtomicBool::new(false),

            inner: BaseAuthenticationToken::new(None),
        };
        token.set_authenticated(false).expect("Cannot set this token to trusted - use constructor which takes a GrantedAuthority list instead");
        token
    }

    pub fn from_builder(builder: &mut UsernamePasswordAuthenticationTokenBuilder) -> Self {
        Self {
            principal: builder.principal.take(),
            credentials: builder.credentials.take(),
            cleared: AtomicBool::new(false),

            inner: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Authentication for UsernamePasswordAuthenticationToken {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        if self.cleared.load(Ordering::Acquire) {
            return None;
        }
        self.credentials.as_ref()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.inner.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.principal.as_ref()
    }

    fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        if is_authenticated {
            return Err(
                "Cannot set this token to trusted - use constructor which takes a GrantedAuthority list instead".into()
            );
        }
        self.inner.set_authenticated(false);

        Ok(())
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.inner.authorities()
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(UsernamePasswordAuthenticationTokenBuilder::with_token(self))
    }

    fn of(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}

impl Principal for UsernamePasswordAuthenticationToken {
    fn name(&self) -> &str {
        self.inner.name()
    }
}

impl CredentialsContainer for UsernamePasswordAuthenticationToken {
    fn erase_credentials(&self) {
        self.inner.erase_credentials();
        self.cleared.store(true, Ordering::Release);
    }
}

impl Clone for UsernamePasswordAuthenticationToken {
    fn clone(&self) -> Self {
        Self {
            principal: self.principal.clone(),
            credentials: self.credentials.clone(),
            cleared: AtomicBool::new(self.cleared.load(Ordering::Acquire)),

            inner: self.inner.clone(),
        }
    }
}

/// A builder of UsernamePasswordAuthenticationToken instances
pub struct UsernamePasswordAuthenticationTokenBuilder {
    principal: Option<AnyObject>,
    credentials: Option<AnyObject>,

    inner: BaseAuthenticationBuilder,
}

impl UsernamePasswordAuthenticationTokenBuilder {
    fn new(token: &mut UsernamePasswordAuthenticationToken) -> Self {
        Self {
            principal: token.principal.take(),
            credentials: token.credentials.take(),

            inner: BaseAuthenticationBuilder::new(token),
        }
    }

    pub fn with_token(token: &UsernamePasswordAuthenticationToken) -> Self {
        Self {
            principal: token.principal.clone(),
            credentials: token.credentials.clone(),

            inner: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for UsernamePasswordAuthenticationTokenBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.inner.authorities(authorities);
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.inner.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn credentials(&mut self, credentials: Option<AuthPrincipal>) {
        self.credentials = credentials;
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.inner.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(UsernamePasswordAuthenticationToken::from_builder(self))
    }
}

impl Deref for UsernamePasswordAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for UsernamePasswordAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for UsernamePasswordAuthenticationTokenBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for UsernamePasswordAuthenticationTokenBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
