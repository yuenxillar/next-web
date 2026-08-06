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
        authentication.is_authenticated() && !self.is_anonymous(authentication)
    }
}

#[derive(Clone, Default)]
pub struct DefaultAuthenticationTrustResolver;

impl AuthenticationTrustResolver for DefaultAuthenticationTrustResolver {
    fn is_anonymous(&self, authentication: Option<&dyn Authentication>) -> bool {
        todo!()
    }

    fn is_remember_me(&self, authentication: Option<&dyn Authentication>) -> bool {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        authentication::{AnonymousAuthenticationToken, RememberMeAuthenticationToken},
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
            "alice",
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        assert!(resolver.is_remember_me(&authentication));
        assert!(resolver.is_authenticated(&authentication));
        assert!(!resolver.is_fully_authenticated(&authentication));
    }
}
