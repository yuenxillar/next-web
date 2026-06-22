use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        testing_authentication_token::TestingAuthenticationToken,
    },
    core::{authentication_error::AuthenticationError, Authentication},
};

#[derive(Clone, Default)]
pub struct TestingAuthenticationProvider;

#[async_trait]
impl AuthenticationProvider for TestingAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<TestingAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only TestingAuthenticationToken is supported",
            ));
        };

        Ok(Arc::new(authentication.clone()))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<TestingAuthenticationToken>()
    }
}
