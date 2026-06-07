use std::{collections::BTreeSet, marker::PhantomData};

use next_web_core::async_trait;

use crate::{
    authorization::{
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        factor_authorization_decision::FactorAuthorizationDecision,
        required_factor::RequiredFactor, required_factor_error::RequiredFactorError,
        AuthorizationResult,
    },
    core::Authentication,
};

pub struct AllRequiredFactorsAuthorizationManager<T> {
    required_factors: Vec<RequiredFactor>,
    _marker: PhantomData<T>,
}

impl<T> AllRequiredFactorsAuthorizationManager<T> {
    pub fn builder() -> AllRequiredFactorsAuthorizationManagerBuilder<T> {
        AllRequiredFactorsAuthorizationManagerBuilder::default()
    }

    pub fn new(required_factors: Vec<RequiredFactor>) -> Self {
        assert!(
            !required_factors.is_empty(),
            "requiredFactors cannot be empty"
        );
        Self {
            required_factors,
            _marker: PhantomData,
        }
    }

    pub fn authorize_factors(
        &self,
        authentication: &dyn Authentication,
    ) -> FactorAuthorizationDecision {
        if !authentication.is_authenticated() {
            return FactorAuthorizationDecision::new(
                self.required_factors
                    .iter()
                    .cloned()
                    .map(RequiredFactorError::create_missing)
                    .collect(),
            );
        }

        let authorities = authentication
            .authorities()
            .into_iter()
            .collect::<BTreeSet<_>>();
        let factor_errors = self
            .required_factors
            .iter()
            .filter_map(|factor| {
                if !authorities.contains(factor.authority()) {
                    Some(RequiredFactorError::create_missing(factor.clone()))
                } else if factor.valid_duration().is_some() {
                    Some(RequiredFactorError::create_expired(factor.clone()))
                } else {
                    None
                }
            })
            .collect();
        FactorAuthorizationDecision::new(factor_errors)
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AllRequiredFactorsAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &mut T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        todo!()
    }
}

pub struct AllRequiredFactorsAuthorizationManagerBuilder<T> {
    required_factors: Vec<RequiredFactor>,
    _marker: PhantomData<T>,
}

impl<T> Default for AllRequiredFactorsAuthorizationManagerBuilder<T> {
    fn default() -> Self {
        Self {
            required_factors: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> AllRequiredFactorsAuthorizationManagerBuilder<T> {
    pub fn require_factor(mut self, required_factor: RequiredFactor) -> Self {
        self.required_factors.push(required_factor);
        self
    }

    pub fn build(self) -> AllRequiredFactorsAuthorizationManager<T> {
        AllRequiredFactorsAuthorizationManager::new(self.required_factors)
    }
}
