use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        testing_authentication_token::TestingAuthenticationToken,
    },
    core::{authentication::Authentication, authentication_error::AuthenticationError},
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

#[cfg(test)]
mod tests {
    use crate::{
        authentication::{
            authentication_provider::AuthenticationProvider,
            testing_authentication_provider::TestingAuthenticationProvider,
            testing_authentication_token::TestingAuthenticationToken,
        },
        core::authority_utils::AuthorityUtils,
    };

    #[tokio::test]
    async fn testing_authentication_provider_accepts_testing_token() {
        let provider = TestingAuthenticationProvider;
        let token = TestingAuthenticationToken::with_authorities(
            "alice",
            Some(String::from("secret")),
            AuthorityUtils::create_authority_list(["ROLE_TEST"]),
        );

        let result = provider.authenticate(&token).await.unwrap();
        assert_eq!(result.get_name(), "alice");
    }
}
