use std::{marker::PhantomData, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

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
    pub fn new<E>(authorization_strategy: E) -> Self
    where
        E: BaseAuthorizationStrategyExt + 'static,
    {
        Self {
            authorization_strategy: Box::new(authorization_strategy),
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
        Self::new(FullyAuthenticatedAuthorizationStrategy::default())
    }

    /// Creates an instance of AuthenticatedAuthorizationManager that determines if the
    /// Authentication is authenticated using remember me.
    pub fn remember_me() -> Self {
        Self::new(RememberMeAuthorizationStrategy::default())
    }

    /// Creates an instance of AuthenticatedAuthorizationManager that determines if the
    /// Authentication is anonymous.
    pub fn anonymous() -> Self {
        Self::new(AnonymousAuthorizationStrategy::default())
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
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        Ok(Some(Arc::new(AuthorizationDecision::new(
            self.authorization_strategy.is_granted(authentication),
        ))))
    }
}

impl<T> Default for AuthenticatedAuthorizationManager<T> {
    fn default() -> Self {
        Self {
            authorization_strategy: Box::new(AuthenticatedAuthorizationStrategy::default()),
            _marker: PhantomData,
        }
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
    base: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for AuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.base
            .trust_resolver
            .is_authenticated(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.base.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct FullyAuthenticatedAuthorizationStrategy {
    base: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for FullyAuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.base
            .trust_resolver
            .is_fully_authenticated(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.base.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct AnonymousAuthorizationStrategy {
    base: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for AnonymousAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.base.trust_resolver.is_anonymous(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.base.set_trust_resolver(trust_resolver);
    }
}

#[derive(Default)]
struct RememberMeAuthorizationStrategy {
    base: BaseAuthorizationStrategy,
}

impl BaseAuthorizationStrategyExt for RememberMeAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.base
            .trust_resolver
            .is_remember_me(Some(authentication))
    }

    fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.base.set_trust_resolver(trust_resolver);
    }
}
