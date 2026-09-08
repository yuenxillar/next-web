use std::{marker::PhantomData, sync::Arc};

use next_web_core::{async_trait, error::BoxError};

use crate::{
    authorization::{
        all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        required_authorities_repository::RequiredAuthoritiesRepository, AuthorizationResult,
    },
    core::Authentication,
};

/// An AuthorizationManager that requires all the authorities returned by a RequiredAuthoritiesRepository implementation.
pub struct RequiredAuthoritiesAuthorizationManager<T> {
    authorities: Arc<dyn RequiredAuthoritiesRepository>,
    _marker: PhantomData<T>,
}

impl<T> RequiredAuthoritiesAuthorizationManager<T> {
    /// Creates a new instance.
    pub fn new(authorities: Arc<dyn RequiredAuthoritiesRepository>) -> Self {
        Self {
            authorities,
            _marker: PhantomData,
        }
    }

    fn find_authorities(&self, authentication: &dyn Authentication) -> Vec<&str> {
        let username = authentication.name();
        self.authorities
            .find_required_authorities(username.as_ref())
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for RequiredAuthoritiesAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Result<Option<Arc<dyn AuthorizationResult>>, BoxError> {
        let authorities = self.find_authorities(authentication);
        if authorities.is_empty() {
            return Ok(Some(Arc::new(AuthorizationDecision::new(true))));
        }

        AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(authorities)
            .authorize(authentication, var)
            .await
    }
}
