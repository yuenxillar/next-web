use std::sync::Arc;

use next_web_core::error::BoxError;

use crate::{
    authentication::{
        anonymous_authentication_token::EMPTY_CREDENTIALS, string_hash, BaseAuthenticationToken,
    },
    core::{granted_authority::GrantedAuthority, Authentication, Principal},
    web::authentication::AuthPrincipal,
};

/// Represents a remembered Authentication.
/// A remembered Authentication must provide a fully valid Authentication, including the GrantedAuthoritys that apply.
#[derive(Clone)]
pub struct RememberMeAuthenticationToken {
    principal: AuthPrincipal,
    key_hash: i32,

    inner: BaseAuthenticationToken,
}

impl RememberMeAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: AuthPrincipal,
        authorities: Option<Vec<Arc<dyn GrantedAuthority>>>,
    ) -> Self {
        assert!(!key.as_ref().trim().is_empty(), "Key cannot be empty");
        assert!(
            !principal
                .as_ref()
                .downcast_ref::<String>()
                .map_or(false, |s| s.trim().is_empty()),
            "Principal cannot be empty"
        );

        let mut inner = BaseAuthenticationToken::new(authorities);
        inner.set_authenticated(true);
        Self {
            principal,
            key_hash: string_hash(key.as_ref()),
            inner,
        }
    }

    /// Private constructor to help with deserialization.
    fn from_key_hash(
        key_hash: i32,
        principal: AuthPrincipal,
        authorities: Option<Vec<Arc<dyn GrantedAuthority>>>,
    ) -> Self {
        let mut inner = BaseAuthenticationToken::new(authorities);
        inner.set_authenticated(true);

        Self {
            principal,
            key_hash,
            inner,
        }
    }

    /// Returns the key hash.
    pub fn get_key_hash(&self) -> i32 {
        self.key_hash
    }

    /// Sets the details about the authentication request.
    pub fn set_details(&mut self, details: Option<AuthPrincipal>) {
        self.inner.set_details(details);
    }

    pub fn key_hash(&self) -> i32 {
        self.key_hash
    }
}

impl Authentication for RememberMeAuthenticationToken {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        Some(&EMPTY_CREDENTIALS)
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        Some(&self.principal)
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.inner.authorities()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.inner.details()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        self.inner.set_authenticated(is_authenticated)
    }
}

impl Principal for RememberMeAuthenticationToken {
    fn name(&self) -> String {
        self.inner.name()
    }
}
