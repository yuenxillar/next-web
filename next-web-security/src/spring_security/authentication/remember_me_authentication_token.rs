use std::{any::TypeId, borrow::Cow, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    authentication::{
        anonymous_authentication_token::EMPTY_STRING, string_hash, BaseAuthenticationToken,
    },
    core::{Authentication, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

/// Represents a remembered Authentication.
/// A remembered Authentication must provide a fully valid Authentication, including the GrantedAuthoritys that apply.
#[derive(Clone)]
pub struct RememberMeAuthenticationToken {
    principal: AuthPrincipal,
    key_hash: i32,

    base: BaseAuthenticationToken,
}

impl RememberMeAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: AuthPrincipal,
        authorities: Option<Vec<Arc<dyn GrantedAuthority>>>,
    ) -> Self {
        assert!(!key.as_ref().trim().is_empty(), "Key cannot be empty");
        assert!(
            !principal.to_string().trim().is_empty(),
            "Principal cannot be empty"
        );

        let mut base = BaseAuthenticationToken::new(authorities);
        base.set_authenticated(true);
        Self {
            principal,
            key_hash: string_hash(key.as_ref()),
            base,
        }
    }

    /// Private constructor to help with deserialization.
    fn from_key_hash(
        key_hash: i32,
        principal: AuthPrincipal,
        authorities: Option<Vec<Arc<dyn GrantedAuthority>>>,
    ) -> Self {
        let mut base = BaseAuthenticationToken::new(authorities);
        base.set_authenticated(true);

        Self {
            principal,
            key_hash,
            base,
        }
    }

    /// Returns the key hash.
    pub fn get_key_hash(&self) -> i32 {
        self.key_hash
    }

    /// Sets the details about the authentication request.
    pub fn set_details(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }

    pub fn key_hash(&self) -> i32 {
        self.key_hash
    }
}

impl Authentication for RememberMeAuthenticationToken {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        Some(&*EMPTY_STRING)
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        Some(&self.principal)
    }

    fn is_authenticated(&self) -> bool {
        true
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        self.base.set_authenticated(is_authenticated)
    }

    fn to_builder(&self) -> Box<dyn crate::core::AuthenticationBuilder> {
        self.base.to_builder()
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for RememberMeAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        self.base.name()
    }
}

impl std::fmt::Debug for RememberMeAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RememberMeAuthenticationToken [Principal={}]",
            self.name()
        )
    }
}
