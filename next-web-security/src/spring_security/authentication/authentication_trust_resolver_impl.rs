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
    fn is_anonymous(&self, authentication: &dyn Authentication) -> bool {
        self.anonymous_type == authentication.of()
    }

    fn is_remember_me(&self, authentication: &dyn Authentication) -> bool {
        self.remember_me_type == authentication.of()
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
