use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_context::{support::MessageSourceAccessor, MessageSource};
use next_web_core::async_trait;

use crate::{
    authentication::{
        authentication_provider::AuthenticationProvider,
        remember_me_authentication_token::RememberMeAuthenticationToken, string_hash,
    },
    core::{
        Authentication, AuthenticationError, AuthenticationErrorKind, NextSecurityMessageSource,
    },
};

/// An AuthenticationProvider implementation that validates RememberMeAuthenticationTokens.
#[derive(Clone)]
pub struct RememberMeAuthenticationProvider {
    messages: MessageSourceAccessor,
    key: String,
}

impl RememberMeAuthenticationProvider {
    /// Creates a new RememberMeAuthenticationProvider with the given key.
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        assert!(!key.trim().is_empty(), "key must have a length");
        Self {
            key,
            messages: NextSecurityMessageSource::get_accessor(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }
}

#[async_trait]
impl AuthenticationProvider for RememberMeAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(remember_me_authentication) =
            (authentication.as_ref() as &dyn Any).downcast_ref::<RememberMeAuthenticationToken>()
        else {
            return Ok(None);
        };

        if string_hash(&self.key) != remember_me_authentication.key_hash() {
            return Err(AuthenticationError::with_kind(
                self.messages.message_or_default(
                    "RememberMeAuthenticationProvider.incorrectKey",
                    None,
                    "The presented RememberMeAuthenticationToken does not contain the expected key",
                ),
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        Ok(Some(authentication.clone()))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<RememberMeAuthenticationToken>()
    }
}
