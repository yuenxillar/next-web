use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        account_status_user_details_exceptions::bad_credentials,
        authentication_provider::AuthenticationProvider,
        remember_me_authentication_token::RememberMeAuthenticationToken,
    },
    core::{authentication::Authentication, authentication_error::AuthenticationError},
};

#[derive(Clone)]
pub struct RememberMeAuthenticationProvider {
    key: String,
}

impl RememberMeAuthenticationProvider {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        assert!(!key.trim().is_empty(), "key must have a length");
        Self { key }
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

#[async_trait]
impl AuthenticationProvider for RememberMeAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<RememberMeAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only RememberMeAuthenticationToken is supported",
            ));
        };

        if java_string_hash(&self.key) != authentication.key_hash() {
            return Err(bad_credentials());
        }

        Ok(Arc::new(authentication.clone()))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<RememberMeAuthenticationToken>()
    }
}

fn java_string_hash(value: &str) -> i32 {
    value
        .chars()
        .fold(0_i32, |acc, ch| acc.wrapping_mul(31).wrapping_add(ch as i32))
}

#[cfg(test)]
mod tests {
    use crate::{
        authentication::{
            authentication_provider::AuthenticationProvider,
            remember_me_authentication_provider::RememberMeAuthenticationProvider,
            remember_me_authentication_token::RememberMeAuthenticationToken,
        },
        core::authority_utils::AuthorityUtils,
    };

    #[tokio::test]
    async fn remember_me_authentication_provider_validates_matching_key() {
        let provider = RememberMeAuthenticationProvider::new("shared-key");
        let token = RememberMeAuthenticationToken::new(
            "shared-key",
            "alice",
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        let result = provider.authenticate(&token).await.unwrap();
        assert!(result.is_remember_me());
    }
}
