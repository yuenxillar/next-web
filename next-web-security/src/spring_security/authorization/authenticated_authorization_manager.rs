use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationTrustResolverImpl,
    authorization::{
        AuthenticationTrustResolver, AuthorizationDecision, AuthorizationManager,
        AuthorizationResult,
    },
    core::Authentication,
};

/// An AuthorizationManager that determines if the current user is authenticated.
pub struct AuthenticatedAuthorizationManager<T> {
    authorization_strategy: Box<dyn BaseAuthorizationStrategyExt>,
    _marker: PhantomData<T>,
}

impl<T> AuthenticatedAuthorizationManager<T> {
    pub fn new(authorization_strategy: Box<dyn BaseAuthorizationStrategyExt>) -> Self {
        Self {
            authorization_strategy,
            _marker: PhantomData,
        }
    }

    /// Sets the AuthenticationTrustResolver to be used. Default is DefaultAuthenticationTrustResolver.
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.authorization_strategy
            .set_trust_resolver(trust_resolver);
    }

    /// Creates an instance of AuthenticatedAuthorizationManager.
    pub fn authenticated() -> Self {
        Self::default()
    }

    /// Creates an instance of AuthenticatedAuthorizationManager that determines if the
    /// Authentication is authenticated without using remember me.
    pub fn fully_authenticated() -> Self {
        Self::new(Box::new(FullyAuthenticatedAuthorizationStrategy::default()))
    }

    /// Creates an instance of AuthenticatedAuthorizationManager that determines if the
    /// Authentication is authenticated using remember me.
    pub fn remember_me() -> Self {
        Self::new(Box::new(RememberMeAuthorizationStrategy::default()))
    }

    /// Creates an instance of AuthenticatedAuthorizationManager that determines if the
    /// Authentication is anonymous.
    pub fn anonymous() -> Self {
        Self::new(Box::new(AnonymousAuthorizationStrategy::default()))
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AuthenticatedAuthorizationManager<T>
where
    T: Send + Sync,
    T: 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        _var: &T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        Some(Box::new(AuthorizationDecision::new(
            self.authorization_strategy.is_granted(authentication),
        )))
    }
}

impl<T> Default for AuthenticatedAuthorizationManager<T> {
    fn default() -> Self {
        Self::authenticated()
    }
}

pub trait BaseAuthorizationStrategyExt
where
    Self: Send + Sync,
{
    fn is_granted(&self, authentication: &dyn Authentication) -> bool;

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>);
}

pub struct BaseAuthorizationStrategy {
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl BaseAuthorizationStrategy {
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }
}

impl Default for BaseAuthorizationStrategy {
    fn default() -> Self {
        Self {
            trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
        }
    }
}

#[derive(Default)]
struct AuthenticatedAuthorizationStrategy {
    inner: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for AuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.inner
            .trust_resolver
            .is_authenticated(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.inner.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct FullyAuthenticatedAuthorizationStrategy {
    inner: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for FullyAuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.inner
            .trust_resolver
            .is_fully_authenticated(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.inner.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct RememberMeAuthorizationStrategy {
    inner: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for RememberMeAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.inner
            .trust_resolver
            .is_remember_me(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.inner.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct AnonymousAuthorizationStrategy {
    inner: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for AnonymousAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.inner.trust_resolver.is_anonymous(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.inner.set_trust_resolver(trust_resolver);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        authentication::{AnonymousAuthenticationToken, RememberMeAuthenticationToken},
        authorization::authentication_trust_resolver::AuthenticationTrustResolver,
        core::{authority_utils::AuthorityUtils, SimpleAuthentication},
    };

    use super::*;

    async fn authorize(
        manager: &AuthenticatedAuthorizationManager<()>,
        authentication: &dyn Authentication,
    ) -> bool {
        let mut var = ();
        manager
            .authorize(authentication, &mut var)
            .await
            .unwrap()
            .is_granted()
    }

    fn authenticated_authentication() -> Arc<dyn Authentication> {
        let mut builder = SimpleAuthentication::default().to_builder();
        builder.principal(Some(Arc::new(String::from("alice"))));
        builder.authenticated(true);
        builder.build()
    }

    fn anonymous_authentication() -> AnonymousAuthenticationToken {
        AnonymousAuthenticationToken::new(
            "key",
            Arc::new(String::from("anonymousUser")),
            AuthorityUtils::create_authority_list(["ROLE_ANONYMOUS"]),
        )
    }

    fn remember_me_authentication() -> RememberMeAuthenticationToken {
        RememberMeAuthenticationToken::new(
            "remember",
            Arc::new(String::from("alice")),
            Some(AuthorityUtils::create_authority_list(["ROLE_USER"])),
        )
    }

    struct AlwaysAnonymousResolver;

    impl AuthenticationTrustResolver for AlwaysAnonymousResolver {
        fn is_anonymous(&self, _authentication: Option<&dyn Authentication>) -> bool {
            true
        }

        fn is_remember_me(&self, _authentication: Option<&dyn Authentication>) -> bool {
            false
        }
    }

    #[tokio::test]
    async fn authenticated_grants_authenticated_user() {
        let manager = AuthenticatedAuthorizationManager::<()>::authenticated();

        assert!(authorize(&manager, authenticated_authentication().as_ref()).await);
    }

    #[tokio::test]
    async fn authenticated_denies_anonymous_user() {
        let manager = AuthenticatedAuthorizationManager::<()>::authenticated();

        assert!(!authorize(&manager, &anonymous_authentication()).await);
    }

    #[tokio::test]
    async fn fully_authenticated_denies_remember_me_user() {
        let manager = AuthenticatedAuthorizationManager::<()>::fully_authenticated();

        assert!(!authorize(&manager, &remember_me_authentication()).await);
    }

    #[tokio::test]
    async fn remember_me_grants_remember_me_user() {
        let manager = AuthenticatedAuthorizationManager::<()>::remember_me();

        assert!(authorize(&manager, &remember_me_authentication()).await);
    }

    #[tokio::test]
    async fn anonymous_grants_anonymous_user() {
        let manager = AuthenticatedAuthorizationManager::<()>::anonymous();

        assert!(authorize(&manager, &anonymous_authentication()).await);
    }

    #[tokio::test]
    async fn set_trust_resolver_overrides_the_default_resolver() {
        let mut manager = AuthenticatedAuthorizationManager::<()>::anonymous();
        manager.set_trust_resolver(Arc::new(AlwaysAnonymousResolver));

        assert!(authorize(&manager, authenticated_authentication().as_ref()).await);
    }
}
