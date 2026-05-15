use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        single_result_authorization_manager::SingleResultAuthorizationManager,
    },
    core::authentication::Authentication,
};

/// An AuthorizationManager that delegates based on a condition evaluated against the Authentication.
pub struct ConditionalAuthorizationManager<T> {
    condition: Arc<dyn Fn(&dyn Authentication) -> bool + Send + Sync>,
    when_true: Arc<dyn AuthorizationManager<T>>,
    when_false: Arc<dyn AuthorizationManager<T>>,
}

impl<T: Send + Sync + 'static> ConditionalAuthorizationManager<T> {
    pub fn new(
        condition: impl Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
        when_true: Arc<dyn AuthorizationManager<T>>,
        when_false: Arc<dyn AuthorizationManager<T>>,
    ) -> Self {
        Self {
            condition: Arc::new(condition),
            when_true,
            when_false,
        }
    }

    pub fn builder(
        condition: impl Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    ) -> ConditionalAuthorizationManagerBuilder<T> {
        ConditionalAuthorizationManagerBuilder::new(condition)
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> AuthorizationManager<T> for ConditionalAuthorizationManager<T> {
    async fn check(
        &self,
        authentication: Box<dyn Authentication>,
        object: T,
    ) -> Option<AuthorizationDecision> {
        let condition_met = (self.condition)(authentication.as_ref());
        if condition_met {
            self.when_true.check(authentication, object).await
        } else {
            self.when_false.check(authentication, object).await
        }
    }
}

pub struct ConditionalAuthorizationManagerBuilder<T: Send + Sync + 'static> {
    condition: Arc<dyn Fn(&dyn Authentication) -> bool + Send + Sync>,
    when_true: Option<Arc<dyn AuthorizationManager<T>>>,
    when_false: Option<Arc<dyn AuthorizationManager<T>>>,
}

impl<T: Send + Sync + 'static> ConditionalAuthorizationManagerBuilder<T> {
    pub fn new(
        condition: impl Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            condition: Arc::new(condition),
            when_true: None,
            when_false: None,
        }
    }

    pub fn when_true(mut self, manager: Arc<dyn AuthorizationManager<T>>) -> Self {
        self.when_true = Some(manager);
        self
    }

    pub fn when_false(mut self, manager: Arc<dyn AuthorizationManager<T>>) -> Self {
        self.when_false = Some(manager);
        self
    }

    pub fn build(self) -> ConditionalAuthorizationManager<T> {
        let when_false = self.when_false.unwrap_or_else(|| {
            Arc::new(SingleResultAuthorizationManager::<T>::permit_all())
        });
        ConditionalAuthorizationManager {
            condition: self.condition,
            when_true: self.when_true.expect("when_true is required"),
            when_false,
        }
    }
}
