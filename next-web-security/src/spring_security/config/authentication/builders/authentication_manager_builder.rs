use next_web_core::async_trait;
use next_web_core::traits::required::Required;
use std::any::Any;
use std::sync::Arc;
use tracing::debug;

use crate::authentication::provider_manager::ProviderManager;
use crate::config::base_configured_security_builder::{
    BaseConfiguredSecurityBuilder, BaseConfiguredSecurityBuilderExt,
};
use crate::config::object_post_processor::ObjectPostProcessor;
use crate::core::{
    userdetails::UserDetailsService,
    Authentication, {AuthenticationError, AuthenticationErrorKind},
};
use crate::{
    authentication::{
        account_status_user_details_exceptions::provider_not_found,
        authentication_event_publisher::{
            AuthenticationEventPublisher, NullAuthenticationEventPublisher,
        },
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

/// SecurityBuilder used to create an AuthenticationManager. Allows for easily building
/// in memory authentication, LDAP authentication, JDBC based authentication, adding UserDetailsService,
/// and adding AuthenticationProvider's.
#[derive(Clone)]
pub struct AuthenticationManagerBuilder {
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    parent_authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    authentication_providers: Vec<Arc<dyn AuthenticationProvider>>,
    default_user_details_service: Option<Arc<dyn UserDetailsService>>,
    erase_credentials: Option<bool>,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,

    base: BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>,
}

impl AuthenticationManagerBuilder {
    /// Creates a new instance
    pub fn new(object_post_processor: Arc<dyn ObjectPostProcessor<&mut dyn Any>>) -> Self {
        Self {
            authentication_manager: Default::default(),
            parent_authentication_manager: Default::default(),
            authentication_providers: Default::default(),
            default_user_details_service: Default::default(),
            erase_credentials: Default::default(),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            base: BaseConfiguredSecurityBuilder::default(),
        }
    }

    pub fn authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> &mut Self {
        self.authentication_manager = Some(authentication_manager);
        self
    }

    /// Allows providing a parent AuthenticationManager that will be tried if this AuthenticationManager
    /// was unable to attempt to authenticate the provided Authentication.
    pub fn parent_authentication_manager<T>(&mut self, authentication_manager: T) -> &mut Self
    where
        T: AuthenticationManager,
        T: 'static,
    {
        let mut boxed_manager = Box::new(authentication_manager) as Box<dyn AuthenticationManager>;
        if let Some(provider_manager) =
            (boxed_manager.as_mut() as &mut dyn Any).downcast_mut::<ProviderManager>()
        {
            self.erase_credentials(provider_manager.is_erase_credentials_after_authentication());
        }
        let authentication_manager = Arc::from(boxed_manager);
        self.parent_authentication_manager = Some(authentication_manager);
        self
    }

    /// Sets the AuthenticationEventPublisher
    pub fn authentication_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthenticationEventPublisher>,
    ) -> &mut Self {
        self.event_publisher = event_publisher;
        self
    }

    pub fn erase_credentials(&mut self, erase_credentials: bool) -> &mut Self {
        self.erase_credentials = Some(erase_credentials);
        self
    }

    /// Gets the default UserDetailsService for the AuthenticationManagerBuilder. The result may be null in some circumstances.
    pub fn default_user_details_service(&self) -> Option<&Arc<dyn UserDetailsService>> {
        self.default_user_details_service.as_ref()
    }

    /// Determines if the AuthenticationManagerBuilder is configured to build a non null AuthenticationManager.
    /// This means that either a non-null parent is specified or at least one AuthenticationProvider has been specified.
    ///
    /// When using SecurityConfigurer instances, the AuthenticationManagerBuilder will not be configured until the
    /// SecurityConfigurer.configure(SecurityBuilder) methods. This means a SecurityConfigurer that is last could
    /// check this method and provide a default configuration in the SecurityConfigurer.configure(SecurityBuilder) method.
    pub fn is_configured(&self) -> bool {
        !self.authentication_providers.is_empty() || self.parent_authentication_manager.is_some()
    }

    pub fn user_details_service(
        &mut self,
        user_details_service: Arc<dyn UserDetailsService>,
    ) -> &mut Self {
        self.default_user_details_service = Some(user_details_service);
        self
    }
}

impl BaseConfiguredSecurityBuilderExt<Arc<dyn AuthenticationManager>, Self>
    for AuthenticationManagerBuilder
{
    fn perform_build(&mut self) -> Arc<dyn AuthenticationManager> {
        if !self.is_configured() {
            debug!("No authenticationProviders and no parentAuthenticationManager defined.");
            return Arc::new(ProviderManager::new(Vec::new()));
        }

        let providers = std::mem::take(&mut self.authentication_providers);
        let mut provider_manager = if let Some(parent) = self.parent_authentication_manager.take() {
            ProviderManager::with_parent(providers, parent)
        } else {
            ProviderManager::new(providers)
        };

        if let Some(erase_credentials) = self.erase_credentials {
            provider_manager.set_erase_credentials_after_authentication(erase_credentials);
        }

        provider_manager.set_authentication_event_publisher(self.event_publisher.clone());

        // post_processing

        Arc::new(provider_manager)
    }
}

impl Default for AuthenticationManagerBuilder {
    fn default() -> Self {
        Self {
            authentication_manager: Default::default(),
            parent_authentication_manager: Default::default(),
            authentication_providers: Default::default(),
            default_user_details_service: Default::default(),
            erase_credentials: Default::default(),
            event_publisher: Arc::new(NullAuthenticationEventPublisher),
            base: BaseConfiguredSecurityBuilder::default(),
        }
    }
}

impl ProviderManagerBuilder<Self> for AuthenticationManagerBuilder {
    /// Add authentication based upon the custom AuthenticationProvider that is passed in.
    /// Since the AuthenticationProvider implementation is unknown, all customizations must be done externally
    /// and the AuthenticationManagerBuilder is returned immediately.
    fn authentication_provider(
        &mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) -> &mut Self {
        self.authentication_providers.push(authentication_provider);
        self
    }
}

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
        &self.base
    }

    fn get_mut_object(
        &mut self,
    ) -> &mut BaseConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &mut self.base
    }
}

struct ProviderAuthenticationManager {
    providers: Vec<Arc<dyn AuthenticationProvider>>,
    parent: Option<Arc<dyn AuthenticationManager>>,
    erase_credentials_after_authentication: bool,
    event_publisher: Arc<dyn AuthenticationEventPublisher>,
}

#[async_trait]
impl AuthenticationManager for ProviderAuthenticationManager {
    async fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        if self.providers.is_empty() {
            return Err(provider_not_found(
                "No AuthenticationProvider registered with the AuthenticationManager",
            ));
        }

        let authentication_type = authentication.of();
        let authentication_arc = authentication.to_builder().build();
        let mut last_error = None;
        let mut parent_attempted = false;
        let mut supported_provider_found = false;
        for provider in &self.providers {
            if !provider.supports(authentication_type) {
                continue;
            }
            supported_provider_found = true;
            let result = provider.authenticate(&authentication_arc).await;

            match result {
                Ok(authentication) => {
                    let authentication = match authentication {
                        Some(value) => self.erase_credentials(value),
                        None => continue,
                    };
                    self.event_publisher
                        .publish_authentication_success(authentication.clone());
                    return Ok(authentication);
                }
                Err(error) => {
                    if error.is_account_status_error() || error.is_internal_service_error() {
                        self.event_publisher.publish_authentication_failure(
                            error.clone(),
                            authentication_arc.clone(),
                        );
                        return Err(error);
                    }
                    last_error = Some(error);
                }
            }
        }

        if let Some(parent) = &self.parent {
            parent_attempted = true;
            match parent.authenticate(authentication).await {
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
                self.event_publisher
                    .publish_authentication_failure(error.clone(), authentication_arc.clone());
            }
            return Err(error);
        }
        if !supported_provider_found {
            let error = provider_not_found(format!(
                "No AuthenticationProvider found for {:?}",
                authentication_type
            ));
            if !parent_attempted {
                self.event_publisher
                    .publish_authentication_failure(error.clone(), authentication_arc.clone());
            }
            return Err(error);
        }
        let error = AuthenticationError::with_kind(
            "No AuthenticationProvider accepted the authentication",
            AuthenticationErrorKind::BadCredentials,
        );
        if !parent_attempted {
            self.event_publisher
                .publish_authentication_failure(error.clone(), authentication_arc);
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
        let mut builder = authentication.to_builder();
        builder.credentials(None);
        builder.build()
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
