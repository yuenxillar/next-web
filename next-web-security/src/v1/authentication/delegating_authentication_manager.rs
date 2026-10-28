use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::account_status_user_details_exceptions::provider_not_found,
    authorization::AuthenticationManager,
    core::{Authentication, AuthenticationError},
};

#[derive(Clone)]
pub struct DelegatingAuthenticationManager {
    delegates: Vec<Arc<dyn AuthenticationManager>>,
}

impl DelegatingAuthenticationManager {
    pub fn new(delegates: Vec<Arc<dyn AuthenticationManager>>) -> Self {
        assert!(!delegates.is_empty(), "delegates cannot be empty");
        Self { delegates }
    }
}

#[async_trait]
impl AuthenticationManager for DelegatingAuthenticationManager {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let mut last_error = None;
        for delegate in &self.delegates {
            match delegate.authenticate(authentication).await {
                Ok(authentication) => return Ok(authentication),
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| provider_not_found("No provider found")))
    }
}
