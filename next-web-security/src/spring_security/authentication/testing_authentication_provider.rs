use std::{any::TypeId, sync::Arc};

use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        testing_authentication_token::TestingAuthenticationToken,
    },
    core::{Authentication, AuthenticationError},
};

#[derive(Clone, Default)]
pub struct TestingAuthenticationProvider;

#[async_trait]
impl AuthenticationProvider for TestingAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        if (authentication.as_ref() as &dyn std::any::Any)
            .downcast_ref::<TestingAuthenticationToken>()
            .is_none()
        {
            return Ok(None);
        }
        Ok(Some(authentication.clone()))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<TestingAuthenticationToken>()
    }
}
