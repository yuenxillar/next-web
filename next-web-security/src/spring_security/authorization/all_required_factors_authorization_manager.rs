use std::{any::Any, collections::HashSet, marker::PhantomData, sync::Arc, time::Instant};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    authorization::{
        authorization_manager::AuthorizationManager,
        factor_authorization_decision::FactorAuthorizationDecision,
        required_factor::{RequiredFactor, RequiredFactorBuilder},
        required_factor_error::RequiredFactorError,
        AuthorizationResult,
    },
    core::{authority::FactorGrantedAuthority, Authentication, GrantedAuthority},
};

/// An AuthorizationManager that determines if the current user is authorized by evaluating if the
/// Authentication contains a FactorGrantedAuthority that is not expired for each RequiredFactor.
pub struct AllRequiredFactorsAuthorizationManager<T> {
    required_factors: Vec<RequiredFactor>,
    _marker: PhantomData<T>,
}

impl<T> AllRequiredFactorsAuthorizationManager<T> {
    /// Creates a new instance.
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

    /// Creates an AuthorizationManager that grants access if at least one
    /// AllRequiredFactorsAuthorizationManager granted. When all managers deny, collects the
    /// unique RequiredFactorErrors from each manager.
    pub fn any_of<I>(managers: I) -> Arc<dyn AuthorizationManager<T>>
    where
        I: IntoIterator<Item = Self>,
        T: Send + Sync + 'static,
    {
        let mut managers = managers.into_iter().collect::<Vec<_>>();
        assert!(managers.len() > 0, "managers cannot contain null elements");
        if managers.len() == 1 {
            return Arc::new(managers.remove(0));
        }

        Arc::new(AnyOfFactorsAuthorizationManager::new(managers))
    }

    /// Given the RequiredFactor and the current FactorGrantedAuthority instances,
    /// returns RequiredFactor or null if granted.
    fn required_factor_error(
        &self,
        required_factor: RequiredFactor,
        current_factors: &[Arc<dyn GrantedAuthority>],
    ) -> Result<Option<RequiredFactorError>, &'static str> {
        let matching_authority = current_factors
            .iter()
            .filter(|granted_authority| {
                granted_authority.authority() == Some(required_factor.authority())
            })
            .next();

        if matching_authority.is_none() {
            return Ok(Some(RequiredFactorError::create_missing(required_factor)?));
        }

        matching_authority.map_or(Ok(None), |authority| {
            match required_factor.valid_duration() {
                Some(duration) => {
                    if let Some(factor_authority) =
                        (authority as &dyn Any).downcast_ref::<FactorGrantedAuthority>()
                    {
                        let now = Instant::now();
                        let expires_at = factor_authority.issued_at() + duration;
                        if now < expires_at {
                            // granted
                            return Ok(None);
                        }
                    }
                }
                None => {
                    // granted (only requires authority to match)
                    return Ok(None);
                }
            };

            // denied (expired or no issuedAt to compare)
            Ok(Some(RequiredFactorError::create_expired(required_factor)?))
        })
    }

    /// Creates a new AllRequiredFactorsAuthorizationManagerBuilder
    pub fn builder() -> AllRequiredFactorsAuthorizationManagerBuilder<T> {
        AllRequiredFactorsAuthorizationManagerBuilder::default()
    }

    /// Extracts all of the FactorGrantedAuthority instances from
    /// Authentication.authorities(). If Authentication is null, or Authentication.is_authenticated() is false, then an empty List is returned.
    fn get_factor_granted_authorities(
        &self,
        authentication: &dyn Authentication,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        if !authentication.is_authenticated() {
            return Vec::new();
        }

        authentication.authorities().to_vec()
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AllRequiredFactorsAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    /// For each RequiredFactor finds the first FactorGrantedAuthority.authority()
    ///  that matches the RequiredFactor.authority(). The FactorGrantedAuthority.issued_at()
    /// must be more recent than RequiredFactor.valid_duration() (if non-null).
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        _var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let current_factor_authorities = self.get_factor_granted_authorities(authentication);

        let mut factor_errors = Vec::with_capacity(2);
        for factor in self.required_factors.iter() {
            if let Some(factor_error) =
                self.required_factor_error(factor.clone(), &current_factor_authorities)?
            {
                factor_errors.push(factor_error);
            }
        }

        Ok(Some(Arc::new(FactorAuthorizationDecision::new(
            factor_errors,
        ))))
    }
}

/// An AuthorizationManager that grants access if at least one
/// AllRequiredFactorsAuthorizationManager granted. When all deny, collects the
/// unique RequiredFactorErrors from each manager.
struct AnyOfFactorsAuthorizationManager<T> {
    managers: Vec<AllRequiredFactorsAuthorizationManager<T>>,
}

impl<T> AnyOfFactorsAuthorizationManager<T> {
    pub fn new(managers: Vec<AllRequiredFactorsAuthorizationManager<T>>) -> Self {
        assert!(managers.len() > 0, "managers cannot contain null elements");
        Self { managers }
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for AnyOfFactorsAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let mut factor_errors = HashSet::new();
        for manager in self.managers.iter() {
            if let Some(decision) = manager.authorize(authentication, var).await? {
                if decision.is_granted() {
                    return Ok(Some(decision));
                }
                if let Some(fad) =
                    (decision.as_ref() as &dyn Any).downcast_ref::<FactorAuthorizationDecision>()
                {
                    factor_errors.extend(fad.factor_errors().iter().cloned());
                }
            }
        }

        Ok(Some(Arc::new(FactorAuthorizationDecision::new(
            factor_errors.into_iter().collect(),
        ))))
    }
}

/// A builder for AllRequiredFactorsAuthorizationManager.
pub struct AllRequiredFactorsAuthorizationManagerBuilder<T> {
    required_factors: Vec<RequiredFactor>,
    _marker: PhantomData<T>,
}

impl<T> AllRequiredFactorsAuthorizationManagerBuilder<T> {
    /// Allows the user to consume the RequiredFactor.Builder that is
    /// passed in and then adds the result to the requireFactor(RequiredFactor).
    pub fn require_factor_with_fn<F>(&mut self, required_factor: F) -> &mut Self
    where
        F: Fn(RequiredFactorBuilder) -> RequiredFactorBuilder,
    {
        let builder = required_factor(RequiredFactorBuilder::default());
        self.require_factor(builder.build())
    }

    /// The RequiredFactor to add.
    pub fn require_factor(&mut self, required_factor: RequiredFactor) -> &mut Self {
        self.required_factors.push(required_factor);
        self
    }

    /// Builds the AllRequiredFactorsAuthorizationManager.
    pub fn build(self) -> AllRequiredFactorsAuthorizationManager<T> {
        assert!(
            !self.required_factors.is_empty(),
            "required_factors cannot be empty"
        );

        AllRequiredFactorsAuthorizationManager::new(self.required_factors)
    }
}

impl<T> Default for AllRequiredFactorsAuthorizationManagerBuilder<T> {
    fn default() -> Self {
        Self {
            required_factors: Vec::new(),
            _marker: PhantomData,
        }
    }
}
