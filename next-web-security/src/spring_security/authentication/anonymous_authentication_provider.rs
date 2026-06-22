use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::{
        account_status_user_details_exceptions::bad_credentials,
        anonymous_authentication_token::AnonymousAuthenticationToken,
        authentication_provider::AuthenticationProvider,
    },
    core::{authentication_error::AuthenticationError, Authentication},
};

#[derive(Clone)]
pub struct AnonymousAuthenticationProvider {
    key: String,
}

impl AnonymousAuthenticationProvider {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        assert!(!key.trim().is_empty(), "A Key is required");
        Self { key }
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

#[async_trait]
impl AuthenticationProvider for AnonymousAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<AnonymousAuthenticationToken>()
        else {
            return Err(AuthenticationError::new(
                "Only AnonymousAuthenticationToken is supported",
            ));
        };

        if java_string_hash(&self.key) != authentication.key_hash() {
            return Err(bad_credentials());
        }

        Ok(Arc::new(authentication.clone()))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<AnonymousAuthenticationToken>()
    }
}

fn java_string_hash(value: &str) -> i32 {
    value.chars().fold(0_i32, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(ch as i32)
    })
}
