use next_web_core::traits::required::Required;
use std::sync::Arc;

use crate::authentication::{
    account_status_user_details_exceptions::provider_not_found,
    authentication_event_publisher::{
        AuthenticationEventPublisher, NullAuthenticationEventPublisher,
    },
    authentication_events::{AuthenticationFailureEvent, AuthenticationSuccessEvent},
    authentication_provider::AuthenticationProvider,
};
use crate::config::base_configured_security_builder::AbstractConfiguredSecurityBuilder;
use crate::core::{
    Authentication,
    authentication_error::{AuthenticationError, AuthenticationErrorKind},
    credentials_container::CredentialsContainer,
    userdetails::user_details_service::UserDetailsService,
};
use crate::{
    authorization::AuthenticationManager,
    config::{
        authentication::provider_manager_builder::ProviderManagerBuilder,
        security_builder::SecurityBuilder,
    },
};

#[derive(Clone)]
pub struct AuthenticationManagerBuilder
where
    Self: Required<AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>>,
{
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    parent_authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    authentication_providers: Vec<Arc<dyn AuthenticationProvider>>,
    default_user_details_service: Option<Arc<dyn UserDetailsService>>,
    erase_credentials: Option<bool>,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,

    abstract_configured_security_builder:
        AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>,
}

impl AuthenticationManagerBuilder {
    pub fn new() -> Self {
        let abstract_configured_security_builder = AbstractConfiguredSecurityBuilder::new(true);
        Self {
            authentication_manager: Default::default(),
            parent_authentication_manager: Default::default(),
            authentication_providers: Default::default(),
            default_user_details_service: Default::default(),
            erase_credentials: Default::default(),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            abstract_configured_security_builder,
        }
    }

    pub fn authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> &mut Self {
        self.authentication_manager = Some(authentication_manager);
        self
    }

    pub fn parent_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> &mut Self {
        self.parent_authentication_manager = Some(authentication_manager);
        self
    }

    pub fn authentication_provider(
        &mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) -> &mut Self {
        self.authentication_providers.push(authentication_provider);
        self
    }

    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> &mut Self {
        self.default_user_details_service = Some(user_details_service);
        self
    }

    pub fn erase_credentials(&mut self, erase_credentials: bool) -> &mut Self {
        self.erase_credentials = Some(erase_credentials);
        self
    }

    pub fn authentication_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthenticationEventPublisher>,
    ) -> &mut Self {
        self.event_publisher = event_publisher;
        self
    }
}

impl ProviderManagerBuilder<Self> for AuthenticationManagerBuilder {}

impl SecurityBuilder<Arc<dyn AuthenticationManager>> for AuthenticationManagerBuilder {
    fn build(&self) -> Arc<dyn AuthenticationManager> {
        if let Some(authentication_manager) = &self.authentication_manager {
            return authentication_manager.clone();
        }

        Arc::new(ProviderAuthenticationManager {
            providers: self.authentication_providers.clone(),
            parent: self.parent_authentication_manager.clone(),
            erase_credentials_after_authentication: self.erase_credentials.unwrap_or(true),
            event_publisher: self.event_publisher.clone(),
        })
    }
}

impl Required<AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>>
    for AuthenticationManagerBuilder
{
    fn get_object(
        &self,
    ) -> &AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &self.abstract_configured_security_builder
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &mut self.abstract_configured_security_builder
    }
}

struct ProviderAuthenticationManager {
    providers: Vec<Arc<dyn AuthenticationProvider>>,
    parent: Option<Arc<dyn AuthenticationManager>>,
    erase_credentials_after_authentication: bool,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,
}

impl AuthenticationManager for ProviderAuthenticationManager {
    fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        if self.providers.is_empty() {
            return Err(provider_not_found(
                "No AuthenticationProvider registered with the AuthenticationManager",
            ));
        }

        let handle = tokio::runtime::Handle::try_current().ok();
        let authentication_type = authentication.authentication_type();
        let mut last_error = None;
        let mut parent_attempted = false;
        let mut supported_provider_found = false;
        for provider in &self.providers {
            if !provider.supports(authentication_type) {
                continue;
            }
            supported_provider_found = true;
            let result = if let Some(handle) = &handle {
                tokio::task::block_in_place(|| handle.block_on(provider.authenticate(authentication)))
            } else {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|err| AuthenticationError::new(err.to_string()))?
                    .block_on(provider.authenticate(authentication))
            };

            match result {
                Ok(authentication) => {
                    let authentication = self.erase_credentials(authentication);
                    self.event_publisher.publish_authentication_success(
                        AuthenticationSuccessEvent::new(authentication.clone()),
                    );
                    return Ok(authentication);
                }
                Err(error) => {
                    if error.is_account_status_error() || error.is_internal_service_error() {
                        self.event_publisher.publish_authentication_failure(
                            AuthenticationFailureEvent::new(authentication, error.clone()),
                        );
                        return Err(error);
                    }
                    last_error = Some(error);
                }
            }
        }

        if let Some(parent) = &self.parent {
            parent_attempted = true;
            match parent.authenticate(authentication) {
                Ok(authentication) => {
                    return Ok(self.erase_credentials(authentication));
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }
        }

        if let Some(error) = last_error {
            if !parent_attempted {
                self.event_publisher.publish_authentication_failure(
                    AuthenticationFailureEvent::new(authentication, error.clone()),
                );
            }
            return Err(error);
        }
        if !supported_provider_found {
            let error = provider_not_found(format!(
                "No AuthenticationProvider found for {authentication_type}"
            ));
            if !parent_attempted {
                self.event_publisher.publish_authentication_failure(
                    AuthenticationFailureEvent::new(authentication, error.clone()),
                );
            }
            return Err(error);
        }
        let error = AuthenticationError::with_kind(
            "No AuthenticationProvider accepted the authentication",
            AuthenticationErrorKind::BadCredentials,
        );
        if !parent_attempted {
            self.event_publisher.publish_authentication_failure(
                AuthenticationFailureEvent::new(authentication, error.clone()),
            );
        }
        Err(error)
    }
}

impl ProviderAuthenticationManager {
    fn erase_credentials(
        &self,
        authentication: Arc<dyn Authentication>,
    ) -> Arc<dyn Authentication> {
        if !self.erase_credentials_after_authentication {
            return authentication;
        }
        if let Some(token) = authentication
            .as_ref()
            .as_any()
            .downcast_ref::<crate::core::username_password_authentication_token::UsernamePasswordAuthenticationToken>()
        {
            let mut token = token.clone();
            CredentialsContainer::erase_credentials(&mut token);
            return Arc::new(token);
        }
        authentication
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use next_web_core::async_trait;

    use crate::{
        authentication::{
            anonymous_authentication_provider::AnonymousAuthenticationProvider,
            anonymous_authentication_token::AnonymousAuthenticationToken,
            authentication_event_publisher::AuthenticationEventPublisher,
            authentication_events::{AuthenticationFailureEvent, AuthenticationSuccessEvent},
            authentication_provider::AuthenticationProvider,
            remember_me_authentication_provider::RememberMeAuthenticationProvider,
            remember_me_authentication_token::RememberMeAuthenticationToken,
        },
        authorization::AuthenticationManager,
        core::{
            Authentication,
            authentication_error::AuthenticationError,
            authority_utils::AuthorityUtils,
            username_password_authentication_token::UsernamePasswordAuthenticationToken,
        },
    };

    use super::AuthenticationManagerBuilder;
    use crate::config::security_builder::SecurityBuilder;

    struct SuccessProvider;

    #[async_trait]
    impl AuthenticationProvider for SuccessProvider {
        async fn authenticate(
            &self,
            authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            let token = authentication
                .as_any()
                .downcast_ref::<UsernamePasswordAuthenticationToken>()
                .expect("expected username/password token");
            Ok(Arc::new(UsernamePasswordAuthenticationToken::authenticated(
                token.get_name(),
                token.get_credentials(),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )))
        }

        fn supports(&self, authentication: &str) -> bool {
            authentication == std::any::type_name::<UsernamePasswordAuthenticationToken>()
        }
    }

    struct UnsupportedProvider;

    #[async_trait]
    impl AuthenticationProvider for UnsupportedProvider {
        async fn authenticate(
            &self,
            _authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            Err(AuthenticationError::new("should not be invoked"))
        }

        fn supports(&self, _authentication: &str) -> bool {
            false
        }
    }

    struct ParentManager;

    impl AuthenticationManager for ParentManager {
        fn authenticate(
            &self,
            authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            Ok(Arc::new(UsernamePasswordAuthenticationToken::authenticated(
                authentication.get_name(),
                None,
                AuthorityUtils::create_authority_list(["ROLE_PARENT"]),
            )))
        }
    }

    #[derive(Default)]
    struct RecordingEventPublisher {
        successes: Mutex<Vec<String>>,
        failures: Mutex<Vec<String>>,
    }

    impl AuthenticationEventPublisher for RecordingEventPublisher {
        fn publish_authentication_success(&self, event: AuthenticationSuccessEvent) {
            if let Ok(mut successes) = self.successes.lock() {
                successes.push(event.authentication().get_name());
            }
        }

        fn publish_authentication_failure(&self, event: AuthenticationFailureEvent) {
            if let Ok(mut failures) = self.failures.lock() {
                failures.push(event.error().get_message().to_string());
            }
        }
    }

    #[test]
    fn provider_manager_erases_credentials_after_authentication() {
        let mut builder = AuthenticationManagerBuilder::new();
        builder.authentication_provider(Arc::new(SuccessProvider));
        let manager = builder.build();

        let request = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let result = manager.authenticate(&request).unwrap();

        assert!(result.is_authenticated());
        assert_eq!(result.get_credentials(), None);
    }

    #[test]
    fn provider_manager_reports_provider_not_found_when_no_provider_supports_token() {
        let mut builder = AuthenticationManagerBuilder::new();
        builder.authentication_provider(Arc::new(UnsupportedProvider));
        let manager = builder.build();

        let request = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let error = match manager.authenticate(&request) {
            Ok(_) => panic!("expected provider not found error"),
            Err(error) => error,
        };

        assert_eq!(
            error.kind(),
            crate::core::authentication_error::AuthenticationErrorKind::ProviderNotFound
        );
    }

    #[test]
    fn provider_manager_falls_back_to_parent_manager() {
        let mut builder = AuthenticationManagerBuilder::new();
        builder.authentication_provider(Arc::new(UnsupportedProvider));
        builder.parent_authentication_manager(Arc::new(ParentManager));
        let manager = builder.build();

        let request = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let result = manager.authenticate(&request).unwrap();

        assert!(result.is_authenticated());
        assert_eq!(result.authorities(), vec![String::from("ROLE_PARENT")]);
    }

    #[test]
    fn provider_manager_publishes_success_and_failure_events() {
        let publisher = Arc::new(RecordingEventPublisher::default());

        let mut success_builder = AuthenticationManagerBuilder::new();
        success_builder.authentication_provider(Arc::new(SuccessProvider));
        success_builder.authentication_event_publisher(publisher.clone());
        let success_manager = success_builder.build();

        let success_request = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("alice")),
            Some(String::from("secret")),
        );
        let _ = success_manager.authenticate(&success_request).unwrap();

        let mut failure_builder = AuthenticationManagerBuilder::new();
        failure_builder.authentication_provider(Arc::new(UnsupportedProvider));
        failure_builder.authentication_event_publisher(publisher.clone());
        let failure_manager = failure_builder.build();

        let failure_request = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(String::from("bob")),
            Some(String::from("secret")),
        );
        let _ = failure_manager.authenticate(&failure_request);

        assert_eq!(publisher.successes.lock().unwrap().as_slice(), ["alice"]);
        assert_eq!(
            publisher.failures.lock().unwrap().as_slice(),
            ["No AuthenticationProvider found for next_web_security::spring_security::core::username_password_authentication_token::UsernamePasswordAuthenticationToken"]
        );
    }

    #[test]
    fn provider_manager_authenticates_anonymous_token_with_matching_key() {
        let mut builder = AuthenticationManagerBuilder::new();
        builder.authentication_provider(Arc::new(AnonymousAuthenticationProvider::new(
            "anonymous-key",
        )));
        let manager = builder.build();

        let request = AnonymousAuthenticationToken::new(
            "anonymous-key",
            "anonymousUser",
            AuthorityUtils::create_authority_list(["ROLE_ANONYMOUS"]),
        );
        let result = manager.authenticate(&request).unwrap();

        assert!(result.is_anonymous());
    }

    #[test]
    fn provider_manager_authenticates_remember_me_token_with_matching_key() {
        let mut builder = AuthenticationManagerBuilder::new();
        builder.authentication_provider(Arc::new(RememberMeAuthenticationProvider::new(
            "remember-key",
        )));
        let manager = builder.build();

        let request = RememberMeAuthenticationToken::new(
            "remember-key",
            "alice",
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );
        let result = manager.authenticate(&request).unwrap();

        assert!(result.is_remember_me());
    }
}
