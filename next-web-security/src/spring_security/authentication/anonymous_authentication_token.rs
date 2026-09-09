use std::{
    any::TypeId,
    borrow::Cow,
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock},
};

use next_web_core::error::BoxError;

use crate::{
    authentication::BaseAuthenticationToken,
    core::{Authentication, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

pub static EMPTY_STRING: LazyLock<AuthPrincipal> = LazyLock::new(|| Arc::new(String::new()));

/// Represents an anonymous Authentication.
#[derive(Clone)]
pub struct AnonymousAuthenticationToken {
    principal: AuthPrincipal,
    key_hash: i32,

    base: BaseAuthenticationToken,
}

impl AnonymousAuthenticationToken {
    pub fn new(
        key: impl AsRef<str>,
        principal: AuthPrincipal,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        assert!(!key.as_ref().trim().is_empty(), "key cannot be  empty");
        assert!(principal.to_string() != "", "principal cannot be  empty");

        assert!(!authorities.is_empty(), "authorities cannot be  empty");

        let mut base = BaseAuthenticationToken::new(Some(authorities));
        base.set_authenticated(true).expect("nothing");

        Self {
            principal,
            key_hash: string_hash(key.as_ref()),
            base,
        }
    }

    pub fn key_hash(&self) -> i32 {
        self.key_hash
    }
}

impl Authentication for AnonymousAuthenticationToken {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        Some(&EMPTY_STRING)
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

impl Principal for AnonymousAuthenticationToken {
    fn name(&self) -> Cow<'_, str> {
        self.base.name()
    }
}

impl std::fmt::Debug for AnonymousAuthenticationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AnonymousAuthenticationToken [Principal={}]",
            self.name()
        )
    }
}

impl Deref for AnonymousAuthenticationToken {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for AnonymousAuthenticationToken {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

pub fn string_hash(value: &str) -> i32 {
    value.chars().fold(0_i32, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(ch as i32)
    })
}
