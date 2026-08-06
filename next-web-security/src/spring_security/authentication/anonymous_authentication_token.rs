use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock},
};

use next_web_core::error::BoxError;

use crate::{
    authentication::BaseAuthenticationToken,
    core::{granted_authority::GrantedAuthority, Authentication, Principal},
    web::authentication::AuthPrincipal,
};

pub static EMPTY_STRING: LazyLock<AuthPrincipal> = LazyLock::new(|| Arc::new(String::new()));

/// Represents an anonymous Authentication.
#[derive(Clone)]
pub struct AnonymousAuthenticationToken {
    principal: AuthPrincipal,
    key_hash: i32,

    inner: BaseAuthenticationToken,
}

impl AnonymousAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: AuthPrincipal,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        assert!(
            !key.as_ref().trim().is_empty(),
            "key cannot be null or empty"
        );
        if let Some(s) = principal.downcast_ref::<String>().map(|s| s.as_str()) {
            assert!(s != "", "principal cannot be null or empty");
        }

        assert!(
            !authorities.is_empty(),
            "authorities cannot be null or empty"
        );

        let mut inner = BaseAuthenticationToken::new(Some(authorities));
        inner.set_authenticated(true);

        Self {
            principal,
            key_hash: string_hash(key.as_ref()),
            inner,
        }
    }

    pub fn key_hash(&self) -> i32 {
        self.key_hash
    }
}

impl Authentication for AnonymousAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.inner.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        Some(&EMPTY_STRING)
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.inner.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        Some(&self.principal)
    }

    fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        self.inner.set_authenticated(is_authenticated)
    }

    fn to_builder(&self) -> Box<dyn crate::core::AuthenticationBuilder> {
        self.inner.to_builder()
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for AnonymousAuthenticationToken {
    fn name(&self) -> &str {
        self.inner.name()
    }
}

impl Deref for AnonymousAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for AnonymousAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub fn string_hash(value: &str) -> i32 {
    value.chars().fold(0_i32, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(ch as i32)
    })
}
