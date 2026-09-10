use std::any::TypeId;
use std::collections::HashSet;
use std::ops::Deref;
use std::sync::Arc;

use next_web_context::support::MessageSourceAccessor;
use next_web_context::{ApplicationEventPublisher, MessageSource};
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{debug, enabled, trace, Level};

use crate::authentication::authentication_details_source::AuthenticationDetailsSource;
use crate::authentication::event::InteractiveAuthenticationSuccessEvent;
use crate::core::context::SecurityContextHolderStrategy;
use crate::core::NextSecurityMessageSource;
use crate::web::authentication::session::SessionAuthenticationStrategy;
use crate::web::authentication::SimpleUrlAuthenticationFailureHandler;
use crate::web::authentication::{
    SavedRequestAwareAuthenticationSuccessHandler, WebAuthenticationDetailsSource,
};
use crate::web::context::{RequestAttributeSecurityContextRepository, SecurityContextRepository};
use crate::{
    authorization::AuthenticationManager,
    core::{context::SecurityContextHolder, Authentication, AuthenticationError},
    web::{
        authentication::{
            authentication_converter::AuthenticationConverter,
            remember_me_services::RememberMeServices, AuthenticationFailureHandler,
            AuthenticationSuccessHandler,
        },
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
    },
};

/// Base processor of browser-based HTTP-based authentication requests.
///
/// # Authentication Process
///
/// The filter requires that you set the `authentication_manager` property. An
/// `AuthenticationManager` is required to process the authentication request tokens
/// created by implementing classes.
///
/// This filter will intercept a request and attempt to perform authentication from that
/// request if the request matches the
/// `set_requires_authentication_request_matcher`.
///
/// Authentication is performed by the
/// `attempt_authentication` method, which must be implemented by subclasses.
///
/// ## Authentication Success
///
/// If authentication is successful, the resulting `Authentication` object will be
/// placed into the `SecurityContext` for the current thread, which is
/// guaranteed to have already been created by an earlier filter.
///
/// The configured `AuthenticationSuccessHandler` will then be called to take the
/// redirect to the appropriate destination after a successful login. The default
/// behaviour is implemented in a `SavedRequestAwareAuthenticationSuccessHandler`
/// which will make use of any `DefaultSavedRequest` set by the
/// `ExceptionTranslationFilter` and redirect the user to the URL contained therein.
/// Otherwise it will redirect to the webapp root "/". You can customize this behaviour
/// by injecting a differently configured instance of this class, or by using a
/// different implementation.
///
/// ## Authentication Failure
///
/// If authentication fails, it will delegate to the configured
/// `AuthenticationFailureHandler` to allow the failure information to be conveyed to
/// the client. The default implementation is `SimpleUrlAuthenticationFailureHandler`,
/// which sends a 401 error code to the client. It may also be configured with a failure
/// URL as an alternative. Again you can inject whatever behaviour you require here.
///
/// ## Event Publication
///
/// If authentication is successful, an `InteractiveAuthenticationSuccessEvent` will
/// be published via the application context. No events will be published if authentication
/// was unsuccessful, because this would generally be recorded via an
/// `AuthenticationManager`-specific application event.
///
/// ## Session Authentication
///
/// The class has an optional `SessionAuthenticationStrategy` which will be invoked
/// immediately after a successful call to `attempt_authentication()`. Different
/// implementations can be injected to enable things like session-fixation attack
/// prevention or to control the number of simultaneous sessions a principal may have.
#[derive(Clone)]
pub struct BaseAuthenticationProcessingFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
    pub(crate) authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    authentication_converter: Option<Arc<dyn AuthenticationConverter>>,

    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    messages: MessageSourceAccessor,
    remember_me_services: Option<Arc<dyn RememberMeServices>>,
    requires_authentication_request_matcher: Arc<dyn RequestMatcher>,
    continue_chain_before_successful_authentication: bool,
    continue_chain_when_no_authentication_result: bool,
    session_strategy: Option<Arc<dyn SessionAuthenticationStrategy>>,
    allow_session_creation: bool,
    success_handler: Arc<dyn AuthenticationSuccessHandler>,
    failure_handler: Arc<dyn AuthenticationFailureHandler>,
    security_context_repository: Arc<dyn SecurityContextRepository>,
    mfa_enabled: bool,
}

impl BaseAuthenticationProcessingFilter {
    /// Creates a new instance with a default filter processes URL.
    pub fn new(default_filter_processes_url: &str) -> Self {
        Self::with_request_matcher(Arc::new(PathPatternRequestMatcher::path_pattern(
            None,
            default_filter_processes_url,
        )))
    }

    pub fn with_request_matcher(
        requires_authentication_request_matcher: Arc<dyn RequestMatcher>,
    ) -> Self {
        BaseAuthenticationProcessingFilter {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            event_publisher: None,
            authentication_details_source: Arc::new(WebAuthenticationDetailsSource::default()),
            authentication_converter: None,
            authentication_manager: None,
            messages: NextSecurityMessageSource::get_accessor(),
            remember_me_services: None,
            requires_authentication_request_matcher,
            continue_chain_before_successful_authentication: false,
            continue_chain_when_no_authentication_result: false,
            session_strategy: None,
            allow_session_creation: true,
            success_handler: Arc::new(SavedRequestAwareAuthenticationSuccessHandler::default()),
            failure_handler: Arc::new(SimpleUrlAuthenticationFailureHandler::default()),
            security_context_repository: Arc::new(
                RequestAttributeSecurityContextRepository::default(),
            ),
            mfa_enabled: false,
        }
    }

    /// Creates a new instance with a default filterProcessesUrl and an
    /// `AuthenticationManager`.
    pub fn with_url_and_manager(
        default_filter_processes_url: &str,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> Self {
        let mut filter = Self::new(default_filter_processes_url);
        filter.set_authentication_manager(authentication_manager);
        filter
    }

    /// Creates a new instance with a `RequestMatcher` and an
    /// `AuthenticationManager`.
    pub fn with_matcher_and_manager(
        requires_authentication_request_matcher: Arc<dyn RequestMatcher>,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> Self {
        let mut filter = Self::with_request_matcher(requires_authentication_request_matcher);
        filter.set_authentication_manager(authentication_manager);
        filter
    }

    fn should_perform_mfa(
        &self,
        current: Option<&dyn Authentication>,
        authentication_result: &dyn Authentication,
    ) -> bool {
        if !self.mfa_enabled {
            return false;
        }
        let current = match current {
            Some(c) => c,
            None => return false,
        };
        if !current.is_authenticated() {
            return false;
        }

        current.name() == authentication_result.name()
    }

    /// Indicates whether this filter should attempt to process a login request for the
    /// current invocation.
    ///
    /// It strips any parameters from the "path" section of the request URL (such as the
    /// jsessionid parameter in `https://host/myapp/index.html;jsessionid=blah`)
    /// before matching against the `filterProcessesUrl` property.
    ///
    /// Subclasses may override for special requirements, such as Tapestry integration.
    fn requires_authentication(
        &self,
        request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> bool {
        if self
            .requires_authentication_request_matcher
            .matches(request)
        {
            return true;
        }
        if enabled!(Level::TRACE) {
            trace!(
                "Did not match request to {:?}",
                self.requires_authentication_request_matcher,
            );
        }

        false
    }

    /// Performs actual authentication.
    ///
    /// The implementation should do one of the following:
    /// 1. Return a populated authentication token for the authenticated user, indicating
    ///    successful authentication
    /// 2. Return None, indicating that the authentication process is still in progress.
    ///    Before returning, the implementation should perform any additional work required
    ///    to complete the process.
    /// 3. Return an `Err(AuthenticationException)` if the authentication process fails
    pub(crate) async fn _attempt_authentication(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let authentication = match self.authentication_converter.as_ref() {
            Some(converter) => match converter.convert(request)? {
                Some(authentication) => authentication,
                None => return Ok(None),
            },
            None => return Ok(None),
        };

        match self.authentication_manager.as_ref() {
            Some(manager) => manager
                .authenticate(authentication.as_ref())
                .await
                .map(Some),
            None => Ok(None),
        }
    }

    /// Default behaviour for successful authentication.
    ///
    /// 1. Sets the successful `Authentication` object on the `SecurityContextHolder`
    /// 2. Informs the configured `RememberMeServices` of the successful login
    /// 3. Fires an `InteractiveAuthenticationSuccessEvent` via the configured
    ///    `ApplicationEventPublisher`
    /// 4. Delegates additional behaviour to the `AuthenticationSuccessHandler`.
    ///
    /// Subclasses can override this method to continue the `FilterChain` after
    /// successful authentication.
    ///
    /// # Arguments
    ///
    /// * `request` - the HTTP request
    /// * `response` - the HTTP response
    /// * `chain` - the filter chain
    /// * `auth_result` - the object returned from the `attempt_authentication` method
    async fn successful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _chain: &dyn HttpFilterChain,
        auth_result: &Arc<dyn Authentication>,
    ) -> Result<(), BoxError> {
        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(auth_result.clone()));

        self.security_context_holder_strategy
            .set_context(context.clone());
        self.security_context_repository
            .save_context(&context, request, response)
            .await;

        if enabled!(Level::DEBUG) {
            debug!("Set SecurityContextHolder to {}", auth_result.name());
        }

        if let Some(remember_me_services) = self.remember_me_services.as_ref() {
            remember_me_services
                .login_success(request, response, auth_result.as_ref())
                .await;
        }

        if let Some(publisher) = self.event_publisher.as_ref() {
            publisher
                .publish_event(Box::new(InteractiveAuthenticationSuccessEvent::new(
                    auth_result.clone(),
                    TypeId::of::<Self>(),
                )))
                .ok();
        }

        self.success_handler
            .on_authentication_success(request, response, auth_result.as_ref());

        Ok(())
    }

    /// Default behaviour for unsuccessful authentication.
    ///
    /// 1. Clears the `SecurityContextHolder`
    /// 2. Stores the exception in the session (if it exists or
    ///    `allowSessionCreation` is set to `true`)
    /// 3. Informs the configured `RememberMeServices` of the failed login
    /// 4. Delegates additional behaviour to the `AuthenticationFailureHandler`.
    fn unsuccessful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        failed: &AuthenticationError,
    ) -> Result<(), BoxError> {
        self.security_context_holder_strategy.clear_context();
        trace!("Failed to process authentication request");
        trace!("Cleared SecurityContextHolder");
        trace!("Handling authentication failure");

        if let Some(remember_me_services) = self.remember_me_services.as_ref() {
            remember_me_services.login_fail(request, response);
        }
        self.failure_handler
            .on_authentication_failure(request, response, failed)?;

        Ok(())
    }

    /// Sets the `AuthenticationConverter` to use.
    ///
    /// # Arguments
    ///
    /// * `authentication_converter` - the converter to use
    ///
    /// # Panics
    ///
    /// Panics if `authentication_converter` is null.
    pub fn set_authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) {
        self.authentication_converter = Some(authentication_converter);
        self.continue_chain_when_no_authentication_result = true;
    }

    /// Returns the `AuthenticationManager`.
    pub fn get_authentication_manager(&self) -> Option<&Arc<dyn AuthenticationManager>> {
        self.authentication_manager.as_ref()
    }

    /// Sets the `AuthenticationManager`.
    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = Some(authentication_manager);
    }

    /// Sets the URL that determines if authentication is required.
    pub fn set_filter_processes_url(&mut self, filter_processes_url: &str) {
        self.set_requires_authentication_request_matcher(Arc::new(
            PathPatternRequestMatcher::path_pattern(None, filter_processes_url),
        ));
    }

    /// Sets the `RequestMatcher` that determines if authentication is required.

    pub fn set_requires_authentication_request_matcher(
        &mut self,
        request_matcher: Arc<dyn RequestMatcher>,
    ) {
        self.requires_authentication_request_matcher = request_matcher;
    }

    /// Returns the `RememberMeServices`.
    pub fn get_remember_me_services(&self) -> Option<&dyn RememberMeServices> {
        self.remember_me_services.as_deref()
    }

    /// Indicates if the filter chain should be continued prior to delegation to
    /// `successful_authentication`, which may be useful in certain environment
    /// (such as Tapestry applications). Defaults to `false`.
    pub fn set_continue_chain_before_successful_authentication(
        &mut self,
        continue_chain_before_successful_authentication: bool,
    ) {
        self.continue_chain_before_successful_authentication =
            continue_chain_before_successful_authentication;
    }

    /// Sets the `ApplicationEventPublisher`.
    ///
    /// # Arguments
    ///
    /// * `event_publisher` - the event publisher to use
    pub fn set_application_event_publisher(
        &mut self,
        event_publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.event_publisher = Some(event_publisher);
    }

    /// Sets the `AuthenticationDetailsSource`.
    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = authentication_details_source;
    }

    /// Sets the `MessageSource`.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }

    /// Returns whether session creation is allowed.
    pub fn get_allow_session_creation(&self) -> bool {
        self.allow_session_creation
    }

    /// Sets whether session creation is allowed.
    pub fn set_allow_session_creation(&mut self, allow_session_creation: bool) {
        self.allow_session_creation = allow_session_creation;
    }

    /// Enables Multi-Factor Authentication (MFA) support.
    pub fn set_mfa_enabled(&mut self, mfa_enabled: bool) {
        self.mfa_enabled = mfa_enabled;
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

    /// Sets the strategy used to handle a successful authentication. By default a
    pub fn set_authentication_success_handler(
        &mut self,
        success_handler: Arc<dyn AuthenticationSuccessHandler>,
    ) {
        self.success_handler = success_handler;
    }

    /// Sets the strategy used to handle a failed authentication.
    pub fn set_authentication_failure_handler(
        &mut self,
        failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.failure_handler = failure_handler;
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
    /// the `SecurityContextHolderStrategy` stored in `SecurityContextHolder`.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    /// Returns the success handler.
    pub fn get_success_handler(&self) -> &dyn AuthenticationSuccessHandler {
        self.success_handler.as_ref()
    }

    /// Returns the failure handler.
    pub fn get_failure_handler(&self) -> &dyn AuthenticationFailureHandler {
        self.failure_handler.as_ref()
    }

    pub fn set_success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = success_handler;
    }

    pub fn set_failure_handler(&mut self, failure_handler: Arc<dyn AuthenticationFailureHandler>) {
        self.failure_handler = failure_handler;
    }

    /// Sets the `RememberMeServices`.
    pub fn set_remember_me_services(&mut self, remember_me_services: Arc<dyn RememberMeServices>) {
        self.remember_me_services = Some(remember_me_services);
    }

    pub async fn do_filter(
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,

        this: &dyn BaseAuthenticationProcessingFilterExt,
    ) -> Result<(), FilterError> {
        if !this.requires_authentication(request, response) {
            filter_chain.do_filter(request, response).await?;

            return Ok(());
        }

        match this.attempt_authentication(request, response).await {
            Ok(Some(mut authentication_result)) => {
                let current = this
                    .security_context_holder_strategy
                    .get_context()
                    .get_authentication();

                if this.should_perform_mfa(current.as_deref(), authentication_result.as_ref()) {
                    let mut builder = authentication_result.as_ref().to_builder();
                    builder.authorities(Box::new(move |authorities| {
                        let new_authorities: HashSet<&str> =
                            authorities.iter().filter_map(|a| a.authority()).collect();

                        let to_extend: Vec<_> = current
                            .as_ref()
                            .expect("current_authority is None")
                            .authorities()
                            .iter()
                            .filter(|ca| {
                                !ca.authority()
                                    .is_some_and(|auth| new_authorities.contains(auth))
                            })
                            .cloned()
                            .collect();

                        authorities.extend(to_extend);
                    }));

                    authentication_result = builder.build();
                }

                if let Some(session_strategy) = this.session_strategy.as_ref() {
                    session_strategy
                        .on_authentication(&authentication_result, request, response)
                        .await?;
                }

                // Authentication success
                if this.continue_chain_before_successful_authentication {
                    filter_chain.do_filter(request, response).await?;
                }

                this.successful_authentication(
                    request,
                    response,
                    filter_chain,
                    &authentication_result,
                )
                .await?;
            }
            Ok(None) => {
                if this.continue_chain_when_no_authentication_result {
                    filter_chain.do_filter(request, response).await?;
                }
                // return immediately as subclass has indicated that it hasn't completed
            }
            Err(err) => {
                tracing::error!(
                    "An internal error occurred while trying to authenticate the user."
                );
                // Authentication failed
                this.unsuccessful_authentication(request, response, &err)?;
            }
        }

        Ok(())
    }
}

impl Named for BaseAuthenticationProcessingFilter {
    fn name(&self) -> &str {
        "BaseAuthenticationProcessingFilter"
    }
}

#[async_trait]
pub trait BaseAuthenticationProcessingFilterExt
where
    Self: Deref<Target = BaseAuthenticationProcessingFilter>,
    Self: Send + Sync,
{
    async fn attempt_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        self._attempt_authentication(request, response).await
    }
}
