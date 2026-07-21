use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_core::async_trait;

use crate::{
    authentication::{
        anonymous_authentication_token::AnonymousAuthenticationToken,
        authentication_provider::AuthenticationProvider, string_hash,
    },
    core::{
        authentication_error::{
            AuthenticationError,
            AuthenticationErrorKind::{self},
        },
        Authentication,
    },
};

/// An AuthenticationProvider implementation that validates AnonymousAuthenticationTokens.
/// To be successfully validated, the AnonymousAuthenticationToken.getKeyHash() must match this class' get_key().
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
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        match (authentication as &dyn Any).downcast_ref::<AnonymousAuthenticationToken>() {
            Some(authentication) => {
                if string_hash(&self.key) != authentication.key_hash() {
                    return Err(AuthenticationError::with_kind("The presented AnonymousAuthenticationToken does not contain the expected key", AuthenticationErrorKind::BadCredentials));
                }
            }
            None => return Ok(None),
        };

        Ok(Some(authentication.clone()))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<AnonymousAuthenticationToken>()
    }
}
