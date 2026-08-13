use std::{marker::PhantomData, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authorization::{
        all_authorities_authorization_manager::AllAuthoritiesAuthorizationManager,
        authorization_decision::AuthorizationDecision, authorization_manager::AuthorizationManager,
        required_authorities_repository::RequiredAuthoritiesRepository, AuthorizationResult,
    },
    core::Authentication,
};

pub struct RequiredAuthoritiesAuthorizationManager<T> {
    authorities: Arc<dyn RequiredAuthoritiesRepository>,
    _marker: PhantomData<T>,
}

impl<T> RequiredAuthoritiesAuthorizationManager<T> {
    pub fn new(authorities: Arc<dyn RequiredAuthoritiesRepository>) -> Self {
        Self {
            authorities,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<T> AuthorizationManager<T> for RequiredAuthoritiesAuthorizationManager<T>
where
    T: Send + Sync + 'static,
{
    // async fn check(
    //     &self,
    //     authentication: Box<dyn Authentication>,
    //     object: T,
    // ) -> Option<AuthorizationDecision> {
    //     let required = self
    //         .authorities
    //         .find_required_authorities(&authentication.get_name());
    //     if required.is_empty() {
    //         return Some(AuthorizationDecision::new(true));
    //     }
    //     AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(required)
    //         .check(authentication, object)
    //         .await
    // }

    async fn authorize(
        &self,
        authentication: &dyn Authentication,
        var: &T,
    ) -> Option<Box<dyn AuthorizationResult>> {
        let required = self
            .authorities
            .find_required_authorities(authentication.name());
        if required.is_empty() {
            return Some(Box::new(AuthorizationDecision::new(true)));
        }

        AllAuthoritiesAuthorizationManager::<T>::has_all_authorities(required)
            .authorize(authentication, var)
            .await
    }
}
