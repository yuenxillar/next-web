use std::sync::Arc;

use crate::{
    authorization::{
        AllRequiredFactorsAuthorizationManagerBuilder, AuthorizationManager,
        ConditionalAuthorizationManager, DefaultAuthorizationManagerFactory, RequiredFactorBuilder,
    },
    core::Authentication,
};

/// Creates common AuthorizationManagerFactory instances.
pub struct AuthorizationManagerFactories;

impl AuthorizationManagerFactories {
    /// Creates a AuthorizationManagerFactories.AdditionalRequiredFactorsBuilder that helps build an
    /// AuthorizationManager to set on DefaultAuthorizationManagerFactory.setAdditionalAuthorization(AuthorizationManager)
    /// for multifactor authentication.
    pub fn multi_factor<T>() -> AdditionalRequiredFactorsBuilder<T>
    where
        T: Clone + Send + Sync + 'static,
    {
        AdditionalRequiredFactorsBuilder::default()
    }
}

/// A builder that allows creating DefaultAuthorizationManagerFactory with
/// additional requirements for RequiredFactors.
pub struct AdditionalRequiredFactorsBuilder<T>
where
    T: Clone + Send + Sync + 'static,
{
    factors: AllRequiredFactorsAuthorizationManagerBuilder<T>,
    when_condition: Option<Box<dyn Fn(&dyn Authentication) -> bool + Send + Sync>>,
}

impl<T> AdditionalRequiredFactorsBuilder<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Apply the required factors only when the given condition is true for the current
    /// Authentication. When the condition is false, no additional factors are required (equivalent to permit-all for the additional authorization). Implemented using
    /// ConditionalAuthorizationManager.when(Predicate).
    pub fn when<C>(mut self, condition: C) -> Self
    where
        C: Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    {
        self.when_condition = Some(Box::new(condition));
        self
    }

    ///  Add additional authorities that will be required.
    pub fn require_factors<'a, I>(self, additional_authorities: I) -> Self
    where
        I: IntoIterator<Item = &'a str>,
    {
        let additional_authorities = additional_authorities.into_iter().collect::<Vec<_>>();
        self.require_factors_witn_fn(move |factors| {
            for authority in additional_authorities.iter() {
                factors.require_factor_with_fn(|factor| factor.authority(*authority));
            }
        })
    }

    pub fn require_factors_witn_fn<F>(mut self, mut factors: F) -> Self
    where
        F: FnMut(&mut AllRequiredFactorsAuthorizationManagerBuilder<T>),
    {
        factors(&mut self.factors);
        self
    }

    pub fn require_factor<F>(mut self, factor: F) -> Self
    where
        F: Fn(RequiredFactorBuilder) -> RequiredFactorBuilder,
    {
        self.factors.require_factor_with_fn(factor);
        self
    }

    /// Builds a DefaultAuthorizationManagerFactory that has the
    /// DefaultAuthorizationManagerFactory.setAdditionalAuthorization(AuthorizationManager) set.
    pub fn build(self) -> DefaultAuthorizationManagerFactory<T> {
        let mut result = DefaultAuthorizationManagerFactory::<T>::default();
        let mut additional_checks =
            Arc::new(self.factors.build()) as Arc<dyn AuthorizationManager<T>>;

        if let Some(condition) = self.when_condition {
            additional_checks = Arc::new(
                ConditionalAuthorizationManager::<T>::when(condition)
                    .when_true(additional_checks)
                    .build(),
            );
        }

        result.set_additional_authorization(Some(additional_checks));
        result
    }
}

impl<T> Default for AdditionalRequiredFactorsBuilder<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            factors: AllRequiredFactorsAuthorizationManagerBuilder::default(),
            when_condition: None,
        }
    }
}
