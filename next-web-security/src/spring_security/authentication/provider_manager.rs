use std::sync::Arc;

use crate::{
    authentication::{
        account_status_user_details_exceptions,
        authentication_event_publisher::{
            AuthenticationEventPublisher, NullAuthenticationEventPublisher,
        },
        authentication_events::{AuthenticationFailureEvent, AuthenticationSuccessEvent},
        authentication_provider::AuthenticationProvider,
    },
    authorization::AuthenticationManager,
    core::{
        Authentication,
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
    },
};

pub struct ProviderManager {
    providers: Vec<Arc<dyn AuthenticationProvider>>,
    parent: Option<Arc<dyn AuthenticationManager>>,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,
    erase_credentials_after_authentication: bool,
}

impl ProviderManager {
    pub fn new(providers: Vec<Arc<dyn AuthenticationProvider>>) -> Self {
        Self {
            providers,
            parent: None,
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            erase_credentials_after_authentication: true,
        }
    }

    pub fn with_parent(
        providers: Vec<Arc<dyn AuthenticationProvider>>,
        parent: Arc<dyn AuthenticationManager>,
    ) -> Self {
        Self {
            providers,
            parent: Some(parent),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            erase_credentials_after_authentication: true,
        }
    }

    pub fn set_authentication_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthenticationEventPublisher>,
    ) {
        self.event_publisher = event_publisher;
    }

    pub fn set_erase_credentials_after_authentication(&mut self, erase: bool) {
        self.erase_credentials_after_authentication = erase;
    }

    pub fn providers(&self) -> &[Arc<dyn AuthenticationProvider>] {
        &self.providers
    }
}

impl AuthenticationManager for ProviderManager {
    fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let auth_type = authentication.authentication_type();
        let mut last_exception: Option<AuthenticationError> = None;
        let mut parent_exception: Option<AuthenticationError> = None;
        let mut result: Option<Arc<dyn Authentication>> = None;
        let mut parent_result: Option<Arc<dyn Authentication>> = None;

        for provider in &self.providers {
            if !provider.supports(auth_type) {
                continue;
            }

            match futures::executor::block_on(provider.authenticate(authentication)) {
                Ok(auth_result) => {
                    result = Some(copy_details(authentication, auth_result));
                    break;
                }
                Err(ex) => match ex.kind() {
                    AuthenticationErrorKind::AccountStatus
                    | AuthenticationErrorKind::InternalService => {
                        prepare_exception(&*self.event_publisher, &ex, authentication);
                        return Err(ex);
                    }
                    _ => {
                        last_exception = Some(ex);
                    }
                },
            }
        }

        // Try parent if no result from providers
        if result.is_none() {
            if let Some(ref parent) = self.parent {
                match parent.authenticate(authentication) {
                    Ok(auth_result) => {
                        parent_result = Some(auth_result.clone());
                        result = Some(auth_result);
                    }
                    Err(ex) => {
                        if ex.kind() != AuthenticationErrorKind::ProviderNotFound {
                            parent_exception = Some(ex.clone());
                            last_exception = Some(ex);
                        }
                    }
                }
            }
        }

        if let Some(ref auth_result) = result {
            // Credential erasure requires interior mutability (Arc<Mutex<>>) in Rust
            // In a full implementation, tokens needing erasure would use Arc<Mutex<dyn Authentication>>
            let _ = self.erase_credentials_after_authentication;

            // Publish success event (only if parent didn't already do it)
            if parent_result.is_none() {
                self.event_publisher
                    .publish_authentication_success(AuthenticationSuccessEvent::new(auth_result.clone()));
            }

            return Ok(auth_result.clone());
        }

        // No result - prepare and throw the last exception
        let final_exception = last_exception.unwrap_or_else(|| {
            account_status_user_details_exceptions::provider_not_found(format!(
                "No AuthenticationProvider found for {}",
                auth_type
            ))
        });

        if parent_exception.is_none() {
            prepare_exception(&*self.event_publisher, &final_exception, authentication);
        }

        Err(final_exception)
    }
}

fn copy_details(
    _source: &dyn Authentication,
    dest: Arc<dyn Authentication>,
) -> Arc<dyn Authentication> {
    dest
}

fn prepare_exception(
    event_publisher: &dyn AuthenticationEventPublisher,
    ex: &AuthenticationError,
    authentication: &dyn Authentication,
) {
    event_publisher.publish_authentication_failure(AuthenticationFailureEvent::new(
        authentication,
        ex.clone(),
    ));
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        authentication::{
            provider_manager::ProviderManager,
            testing_authentication_provider::TestingAuthenticationProvider,
            testing_authentication_token::TestingAuthenticationToken,
        },
        authorization::AuthenticationManager,
        core::{authority_utils::AuthorityUtils, Authentication},
    };

    #[test]
    fn test_provider_manager_authenticates_with_supported_provider() {
        let provider = Arc::new(TestingAuthenticationProvider);
        let manager = ProviderManager::new(vec![provider]);

        let auth = TestingAuthenticationToken::with_authorities(
            "alice",
            Some(String::from("secret")),
            AuthorityUtils::create_authority_list(["ROLE_TEST"]),
        );

        let result = manager.authenticate(&auth);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().get_name(), "alice");
    }

    #[test]
    fn test_provider_manager_fails_when_no_provider_supports() {
        let manager = ProviderManager::new(vec![]);

        let auth = TestingAuthenticationToken::new("alice", Some(String::from("secret")));
        let result = manager.authenticate(&auth);
        assert!(result.is_err());
    }
}
