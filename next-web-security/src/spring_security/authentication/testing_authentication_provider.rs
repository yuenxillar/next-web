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
        // let Some(authentication) = authentication
        //     .as_any()
        //     .downcast_ref::<TestingAuthenticationToken>()
        // else {
        //     return Err(AuthenticationError::new(
        //         "Only TestingAuthenticationToken is supported",
        //     ));
        // };

        // Ok(Arc::new(authentication.clone()))
        //
        todo!()
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<TestingAuthenticationToken>()
    }
}
