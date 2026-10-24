use std::any::TypeId;

use crate::{
    authentication::{AnonymousAuthenticationToken, RememberMeAuthenticationToken},
    authorization::AuthenticationTrustResolver,
    core::Authentication,
};

/// Basic implementation of AuthenticationTrustResolver.
/// Make trust decisions based on whether the authentication passed is an instance of a defined structure.
#[derive(Clone)]
pub struct AuthenticationTrustResolverImpl {
    anonymous_type: TypeId,
    remember_me_type: TypeId,
}

impl AuthenticationTrustResolverImpl {
    pub fn get_anonymous_type(&self) -> TypeId {
        self.anonymous_type
    }

    pub fn get_remember_me_type(&self) -> TypeId {
        self.remember_me_type
    }

    pub fn set_anonymous_type(&mut self, anonymous_type: TypeId) {
        self.anonymous_type = anonymous_type;
    }

    pub fn set_remember_me_type(&mut self, remember_me_type: TypeId) {
        self.remember_me_type = remember_me_type;
    }
}
impl AuthenticationTrustResolver for AuthenticationTrustResolverImpl {
    fn is_anonymous(&self, authentication: Option<&dyn Authentication>) -> bool {
        authentication
            .map(|authentication| self.anonymous_type == authentication.of())
            .unwrap_or(false)
    }

    fn is_remember_me(&self, authentication: Option<&dyn Authentication>) -> bool {
        authentication
            .map(|authentication| self.remember_me_type == authentication.of())
            .unwrap_or(false)
    }
}

impl Default for AuthenticationTrustResolverImpl {
    fn default() -> Self {
        Self {
            anonymous_type: TypeId::of::<AnonymousAuthenticationToken>(),
            remember_me_type: TypeId::of::<RememberMeAuthenticationToken>(),
        }
    }
}
