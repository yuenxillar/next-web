use std::any::TypeId;

use crate::authentication::{AnonymousAuthenticationToken, RememberMeAuthenticationToken};
use crate::core::Authentication;

pub trait AuthenticationTrustResolver
where
    Self: Send + Sync,
{
    fn is_anonymous(&self, authentication: Option<&dyn Authentication>) -> bool;

    fn is_remember_me(&self, authentication: Option<&dyn Authentication>) -> bool;

    fn is_fully_authenticated(&self, authentication: Option<&dyn Authentication>) -> bool {
        self.is_authenticated(authentication) && !self.is_remember_me(authentication)
    }

    fn is_authenticated(&self, authentication: Option<&dyn Authentication>) -> bool {
        authentication
            .map(|authentication| authentication.is_authenticated())
            .unwrap_or(false)
            && !self.is_anonymous(authentication)
    }
}

#[derive(Clone, Default)]
pub struct DefaultAuthenticationTrustResolver;

impl AuthenticationTrustResolver for DefaultAuthenticationTrustResolver {
    fn is_anonymous(&self, authentication: Option<&dyn Authentication>) -> bool {
        authentication
            .map(|authentication| {
                authentication.of() == TypeId::of::<AnonymousAuthenticationToken>()
            })
            .unwrap_or(false)
    }

    fn is_remember_me(&self, authentication: Option<&dyn Authentication>) -> bool {
        authentication
            .map(|authentication| {
                authentication.of() == TypeId::of::<RememberMeAuthenticationToken>()
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        authentication::RememberMeAuthenticationToken,
        authorization::authentication_trust_resolver::{
            AuthenticationTrustResolver, DefaultAuthenticationTrustResolver,
        },
        core::authority_utils::AuthorityUtils,
    };

    #[test]
    fn trust_resolver_detects_remember_me_authentication() {
        let resolver = DefaultAuthenticationTrustResolver;
        let authentication = RememberMeAuthenticationToken::new(
            "remember",
            std::sync::Arc::new(String::from("alice")),
            Some(AuthorityUtils::create_authority_list(["ROLE_USER"])),
        );

        assert!(resolver.is_remember_me(Some(&authentication)));
        assert!(resolver.is_authenticated(Some(&authentication)));
        assert!(!resolver.is_fully_authenticated(Some(&authentication)));
    }
}
