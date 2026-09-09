use std::{any::TypeId, sync::Arc};

use next_web_context::ApplicationEventPublisher;
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::debug;

use crate::{
    authentication::event::InteractiveAuthenticationSuccessEvent,
    authorization::AuthenticationManager,
    core::context::{SecurityContextHolder, SecurityContextHolderStrategy},
    web::{
        authentication::{
            remember_me_services::RememberMeServices, session::SessionAuthenticationStrategy,
            AuthenticationSuccessHandler,
        },
        context::{HttpSessionSecurityContextRepository, SecurityContextRepository},
    },
};

/// Detects if there is no `Authentication` object in the `SecurityContext`,
/// and populates the context with a remember-me authentication token if a
/// `RememberMeServices` implementation so requests.
///
/// Concrete `RememberMeServices` implementations will have their
/// `RememberMeServices::auto_login` method called by this filter. If this method
/// returns a non-null `Authentication` object, it will be passed to the
/// `AuthenticationManager`, so that any authentication-specific behaviour can be
/// achieved. The resulting `Authentication` (if successful) will be placed into
/// the `SecurityContext`.
///
/// If authentication is successful, an `InteractiveAuthenticationSuccessEvent` will
/// be published to the application context. No events will be published if authentication
/// was unsuccessful, because this would generally be recorded via an
/// `AuthenticationManager`-specific application event.
///
/// Normally the request will be allowed to proceed regardless of whether authentication
/// succeeds or fails. If some control over the destination for authenticated users is
/// required, an `AuthenticationSuccessHandler` can be injected.
#[derive(Clone)]
pub struct RememberMeAuthenticationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
    success_handler: Option<Arc<dyn AuthenticationSuccessHandler>>,
    authentication_manager: Arc<dyn AuthenticationManager>,
    remember_me_services: Arc<dyn RememberMeServices>,
    security_context_repository: Arc<dyn SecurityContextRepository>,
    session_strategy: Option<Arc<dyn SessionAuthenticationStrategy>>,
}

impl RememberMeAuthenticationFilter {
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        remember_me_services: Arc<dyn RememberMeServices>,
    ) -> Self {
        Self {
            authentication_manager,
            remember_me_services,

            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            security_context_repository: Arc::new(HttpSessionSecurityContextRepository::default()),
            event_publisher: None,
            success_handler: None,
            session_strategy: None,
        }
    }

    pub fn after_properties_set(&self) {}

    pub fn set_application_event_publisher(
        &mut self,
        event_publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.event_publisher = Some(event_publisher);
    }

    /// Returns the `RememberMeServices` implementation.
    pub fn get_remember_me_services(&self) -> &Arc<dyn RememberMeServices> {
        &self.remember_me_services
    }

    /// successfully authenticated. By default, the filter will just allow the current
    /// request to proceed, but if an `AuthenticationSuccessHandler` is set, it will
    /// be invoked and the `do_filter()` method will return immediately, thus allowing
    /// the application to redirect the user to a specific URL, regardless of what the
    /// original request was for.
    pub fn set_authentication_success_handler(
        &mut self,
        authentication_success_handler: Arc<dyn AuthenticationSuccessHandler>,
    ) {
        self.success_handler = Some(authentication_success_handler);
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// authentication success. The default action is not to save the
    /// `SecurityContext`.
    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = security_context_repository;
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use
    /// the `SecurityContextHolderStrategy` stored in `SecurityContextHolder`
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    /// The session handling strategy which will be invoked immediately after an
    /// authentication request is successfully processed by the
    /// `AuthenticationManager`. Used, for example, to handle changing of the
    /// session identifier to prevent session fixation attacks.
    pub fn set_session_authentication_strategy(
        &mut self,
        session_strategy: Arc<dyn SessionAuthenticationStrategy>,
    ) {
        self.session_strategy = Some(session_strategy);
    }
}

#[async_trait]
impl HttpFilter for RememberMeAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if SecurityContextHolder::get_context()
            .get_authentication()
            .is_some()
        {
            debug!("SecurityContextHolder not populated with remember-me token, as it already contained an authentication");
            return filter_chain.do_filter(request, response).await;
        }

        if let Some(rememberme_token) = self
            .remember_me_services
            .auto_login(request, response)
            .await
        {
            let result: Result<(), BoxError> = async {
                // Authenticate the remember-me token via the AuthenticationManager
                let remember_me_auth = self
                    .authentication_manager
                    .authenticate(rememberme_token.as_ref())
                    .await?;
                if let Some(session_strategy) = self.session_strategy.as_ref() {
                    session_strategy
                        .on_authentication(&remember_me_auth, request, response)
                        .await?;
                }
                // Store to SecurityContextHolder
                let ctx = SecurityContextHolder::create_empty_context();
                ctx.set_authentication(Some(remember_me_auth.clone()));
                self.security_context_holder_strategy
                    .set_context(ctx.clone());

                // self.on_successful_authentication(request, response, remember_me_auth);
                debug!("SecurityContextHolder populated with remember-me token");
                self.security_context_repository
                    .save_context(&ctx, request, response)
                    .await;
                if let Some(event_publisher) = self.event_publisher.as_ref() {
                    event_publisher.publish_event(Box::new(
                        InteractiveAuthenticationSuccessEvent::new(
                            remember_me_auth.clone(),
                            TypeId::of::<Self>(),
                        ),
                    ))?;
                }

                if let Some(success_handler) = self.success_handler.as_ref() {
                    success_handler.on_authentication_success(
                        request,
                        response,
                        remember_me_auth.as_ref(),
                    );
                }

                Ok(())
            }
            .await;

            if let Err(_err) = result {
                debug!(
                        "SecurityContextHolder not populated with remember-me token, as AuthenticationManager rejected Authentication returned by RememberMeServices: [remembermeToken]; invalidating remember-me token "
                    );
                self.remember_me_services.login_fail(request, response);
                // self.on_unsuccessful_authentication(request, response, err);
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for RememberMeAuthenticationFilter {
    fn name(&self) -> &str {
        "RememberMeAuthenticationFilter"
    }
}
