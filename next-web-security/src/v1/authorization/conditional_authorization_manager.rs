use std::sync::Arc;

use next_web_core::{async_trait, error::BoxError};

use crate::{
    authorization::{
        authorization_manager::AuthorizationManager,
        single_result_authorization_manager::SingleResultAuthorizationManager, AuthorizationResult,
    },
    core::Authentication,
};

type Predicate = Arc<dyn Fn(&dyn Authentication) -> bool + Send + Sync>;

/// An AuthorizationManager that delegates to one of two AuthorizationManager instances based on
/// a condition evaluated against the current Authentication.
///
/// When authorize(Supplier, Object) is invoked, the condition is evaluated. If the Authentication
/// is non-null and the condition returns true, the whenTrue manager is used; otherwise the whenFalse
/// manager is used.
pub struct ConditionalAuthorizationManager<T> {
    condition: Predicate,
    when_true: Arc<dyn AuthorizationManager<T>>,
    when_false: Arc<dyn AuthorizationManager<T>>,
}

impl<T: Send + Sync + 'static> ConditionalAuthorizationManager<T> {
    /// Creates a ConditionalAuthorizationManager that delegates to whenTrue when the
    /// condition holds for the current Authentication, and to whenFalse otherwise.
    pub fn new<C>(
        condition: C,
        when_true: Arc<dyn AuthorizationManager<T>>,
        when_false: Arc<dyn AuthorizationManager<T>>,
    ) -> Self
    where
        C: Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    {
        Self {
            condition: Arc::new(condition),
            when_true,
            when_false,
        }
    }

    /// Creates a builder for a ConditionalAuthorizationManager with the given condition.
    pub fn when<C>(condition: C) -> ConditionalAuthorizationManagerBuilder<T>
    where
        C: Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    {
        ConditionalAuthorizationManagerBuilder::new(condition)
    }

    pub fn builder(
        condition: impl Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    ) -> ConditionalAuthorizationManagerBuilder<T> {
        ConditionalAuthorizationManagerBuilder::new(condition)
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> AuthorizationManager<T> for ConditionalAuthorizationManager<T> {
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        if (self.condition)(authentication) {
            return self.when_true.authorize(authentication, var).await;
        }

        self.when_false.authorize(authentication, var).await
    }
}

/// Creates a builder for a ConditionalAuthorizationManager with the given condition.
pub struct ConditionalAuthorizationManagerBuilder<T: Send + Sync + 'static> {
    condition: Arc<dyn Fn(&dyn Authentication) -> bool + Send + Sync>,
    when_true: Option<Arc<dyn AuthorizationManager<T>>>,
    when_false: Option<Arc<dyn AuthorizationManager<T>>>,
}

impl<T: Send + Sync + 'static> ConditionalAuthorizationManagerBuilder<T> {
    pub fn new<C>(condition: C) -> Self
    where
        C: Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    {
        Self {
            condition: Arc::new(condition),
            when_true: None,
            when_false: None,
        }
    }

    /// Sets the AuthorizationManager to use when the condition is true.
    pub fn when_true(mut self, manager: Arc<dyn AuthorizationManager<T>>) -> Self {
        self.when_true = Some(manager);
        self
    }

    /// Sets the AuthorizationManager to use when the condition is false. Defaults to SingleResultAuthorizationManager.permit_all() if not set.
    pub fn when_false(mut self, manager: Arc<dyn AuthorizationManager<T>>) -> Self {
        self.when_false = Some(manager);
        self
    }

    pub fn build(self) -> ConditionalAuthorizationManager<T> {
        let when_false = self
            .when_false
            .unwrap_or_else(|| Arc::new(SingleResultAuthorizationManager::<T>::permit_all()));
        ConditionalAuthorizationManager {
            condition: self.condition,
            when_true: self.when_true.expect("when_true is required"),
            when_false,
        }
    }
}
