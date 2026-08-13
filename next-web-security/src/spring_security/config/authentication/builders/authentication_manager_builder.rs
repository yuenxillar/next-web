use next_web_core::traits::required::Required;
use std::any::Any;
use std::sync::Arc;

use crate::config::base_configured_security_builder::BaseConfiguredSecurityBuilder;
use crate::core::UsernamePasswordAuthenticationToken;
use crate::core::{
    authentication_error::{AuthenticationError, AuthenticationErrorKind},
    userdetails::UserDetailsService,
    Authentication, CredentialsContainer,
};
use crate::{
    authentication::{
        account_status_user_details_exceptions::provider_not_found,
        authentication_event_publisher::{
            AuthenticationEventPublisher, NullAuthenticationEventPublisher,
        },
        authentication_events::{AuthenticationFailureEvent, AuthenticationSuccessEvent},
        AuthenticationProvider,
    },
    config::{security_configurer::SecurityConfigurer, web::builders::HttpSecurity},
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};
use crate::{
    authorization::AuthenticationManager,
    config::{
        authentication::provider_manager_builder::ProviderManagerBuilder,
        security_builder::SecurityBuilder,
    },
};

#[derive(Clone)]
pub struct AuthenticationManagerBuilder {
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    parent_authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    authentication_providers: Vec<Arc<dyn AuthenticationProvider>>,
    default_user_details_service: Option<Arc<dyn UserDetailsService>>,
    erase_credentials: Option<bool>,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,

    base_configured_security_builder:
        BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>,
}

impl AuthenticationManagerBuilder {
    pub fn new() -> Self {
        let base_configured_security_builder = BaseConfiguredSecurityBuilder::new();
        Self {
            authentication_manager: Default::default(),
            parent_authentication_manager: Default::default(),
            authentication_providers: Default::default(),
            default_user_details_service: Default::default(),
            erase_credentials: Default::default(),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            base_configured_security_builder,
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
    fn build(&mut self) -> Arc<dyn AuthenticationManager> {
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

impl Required<BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>>
    for AuthenticationManagerBuilder
{
    fn get_object(&self) -> &BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &self.base_configured_security_builder
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &mut self.base_configured_security_builder
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
                tokio::task::block_in_place(|| {
                    handle.block_on(provider.authenticate(authentication))
                })
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
            self.event_publisher
                .publish_authentication_failure(AuthenticationFailureEvent::new(
                    authentication,
                    error.clone(),
                ));
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
        if let Some(token) = (authentication.as_ref() as &dyn Any)
            .downcast_ref::<UsernamePasswordAuthenticationToken>()
        {
            let mut token = token.clone();
            CredentialsContainer::erase_credentials(&mut token);
            return Arc::new(token);
        }
        authentication
    }
}

impl SecurityConfigurer<DefaultSecurityFilterChain, HttpSecurity> for AuthenticationManagerBuilder {
    fn init(&mut self, http: &mut HttpSecurity) {
        todo!()
    }

    fn configure(&mut self, http: &mut HttpSecurity) {
        todo!()
    }
}
