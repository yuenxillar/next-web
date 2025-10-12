use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authorization::{
        authentication_trust_resolver::AuthenticationTrustResolver,
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
    },
    core::authentication::Authentication,
};

pub struct AuthenticatedAuthorizationManager {
    authorization_strategy: Arc<dyn AuthorizationStrategy>,
}

impl AuthenticatedAuthorizationManager {
    pub fn authenticated() -> Self {
        // AuthenticatedAuthorizationManager {
        //     authorization_strategy: Arc::new(AuthenticatedAuthorizationStrategy::new()),
        // }
        todo!()
    }

    pub fn fully_authenticated() -> Self {
        AuthenticatedAuthorizationManager {
            authorization_strategy: Arc::new(FullyAuthenticatedAuthorizationStrategy::new()),
        }
    }

    pub fn remember_me() -> Self {
        AuthenticatedAuthorizationManager {
            authorization_strategy: Arc::new(RememberMeAuthorizationStrategy::new()),
        }
    }

    pub fn anonymous() -> Self {
        AuthenticatedAuthorizationManager {
            authorization_strategy: Arc::new(AnonymousAuthorizationStrategy::new()),
        }
    }
}

impl AuthenticatedAuthorizationManager {
    // pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn TrustResolver>) {

    // }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AuthenticatedAuthorizationManager
where
    T: Send + Sync,
    T: 'static,
{
    async fn check(
        &self,
        authentication: Box<dyn Authentication>,
        _object: T,
    ) -> Option<AuthorizationDecision> {
        let granted = self
            .authorization_strategy
            .is_granted(authentication.as_ref());
        Some(AuthorizationDecision::new(granted))
    }
}

trait AuthorizationStrategy: Send + Sync {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool;
}

pub struct AbstractAuthorizationStrategy {
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl AbstractAuthorizationStrategy {
    pub fn new(trust_resolver: impl AuthenticationTrustResolver + 'static) -> Self {
        Self {
            trust_resolver: Arc::new(trust_resolver),
        }
    }
}

struct AuthenticatedAuthorizationStrategy {
    abstract_authorization_strategy: AbstractAuthorizationStrategy,
}

impl AuthenticatedAuthorizationStrategy {
    pub fn new(authorization_strategy: impl AuthenticationTrustResolver + 'static) -> Self {
        Self {
            abstract_authorization_strategy: AbstractAuthorizationStrategy::new(
                authorization_strategy,
            ),
        }
    }
}

impl AuthorizationStrategy for AuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.abstract_authorization_strategy
            .trust_resolver
            .is_authenticated(authentication)
    }
}

struct FullyAuthenticatedAuthorizationStrategy {
    abstract_authorization_strategy: AbstractAuthorizationStrategy,
}

impl FullyAuthenticatedAuthorizationStrategy {
    pub fn new() -> Self {
        Self { abstract_authorization_strategy: todo!() }
    }
}

impl AuthorizationStrategy for FullyAuthenticatedAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.abstract_authorization_strategy
            .trust_resolver
            .is_fully_authenticated(authentication)
    }
}

struct RememberMeAuthorizationStrategy {
    abstract_authorization_strategy: AbstractAuthorizationStrategy,
}

impl RememberMeAuthorizationStrategy {
    pub fn new() -> Self {
        Self { abstract_authorization_strategy: todo!() }
    }
}

impl AuthorizationStrategy for RememberMeAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.abstract_authorization_strategy
            .trust_resolver
            .is_remember_me(authentication)
    }
}

struct AnonymousAuthorizationStrategy {
    abstract_authorization_strategy: AbstractAuthorizationStrategy,
}

impl AnonymousAuthorizationStrategy {
    pub fn new() -> Self {
        Self { abstract_authorization_strategy: todo!() }
    }
}

impl AuthorizationStrategy for AnonymousAuthorizationStrategy {
    fn is_granted(&self, authentication: &dyn Authentication) -> bool {
        self.abstract_authorization_strategy
            .trust_resolver
            .is_anonymous(authentication)
    }
}
