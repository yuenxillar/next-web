use std::marker::PhantomData;

use crate::{
    authorization::default_authorization_manager_factory::DefaultAuthorizationManagerFactory,
    core::Authentication,
};

/// Creates common AuthorizationManagerFactory instances.
pub struct AuthorizationManagerFactories;

impl AuthorizationManagerFactories {
    /// Create a builder for multi-factor authentication.
    pub fn multi_factor<T: Clone + Send + Sync + 'static>() -> MultiFactorBuilder<T> {
        MultiFactorBuilder::new()
    }
}

/// Builder for configuring additional required factors.
pub struct MultiFactorBuilder<T: Clone + Send + Sync + 'static> {
    required_factors: Vec<String>,
    when_condition: Option<Box<dyn Fn(&dyn Authentication) -> bool + Send + Sync>>,
    _marker: PhantomData<T>,
}

impl<T: Clone + Send + Sync + 'static> MultiFactorBuilder<T> {
    fn new() -> Self {
        Self {
            required_factors: vec![],
            when_condition: None,
            _marker: PhantomData,
        }
    }

    pub fn when(
        mut self,
        condition: impl Fn(&dyn Authentication) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.when_condition = Some(Box::new(condition));
        self
    }

    pub fn require_factors(mut self, factors: Vec<String>) -> Self {
        self.required_factors = factors;
        self
    }

    pub fn build(self) -> DefaultAuthorizationManagerFactory<T> {
        let mut factory = DefaultAuthorizationManagerFactory::<T>::default();
        let _ = self.required_factors;
        let _ = self.when_condition;
        factory
    }
}
