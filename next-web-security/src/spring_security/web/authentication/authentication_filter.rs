use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use tracing::{debug, trace};

use crate::{
    authorization::AuthenticationManager,
    core::{
        authentication_error::AuthenticationError,
        context::{security_context_holder::SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication,
    },
    web::{
        authentication::{
            authentication_converter::AuthenticationConverter,
            authentication_failure_handler::AuthenticationFailureHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
            saved_request_aware_authentication_success_handler::SavedRequestAwareAuthenticationSuccessHandler,
            simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
        },
        context::SecurityContextRepository,
        util::matcher::{AnyRequestMatcher, RequestMatcher},
    },
};

/// Global counter used to generate unique per-instance filter keys.
/// This mirrors `OncePerRequestFilter`'s behavior where each instance
/// gets a unique attribute name based on `System.identityHashCode(this)`.
static FILTER_INSTANCE_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A closure-based resolver that maps an `HttpRequest` to an `AuthenticationManager`.
type AuthenticationManagerResolverFn =
    Arc<dyn Fn(&dyn HttpRequest) -> Arc<dyn AuthenticationManager> + Send + Sync>;

/// A {@link Filter} that performs authentication of a particular request. An outline of
/// the logic:
///
/// * A request comes in and if it does not match
///   {@link #setRequestMatcher(RequestMatcher)}, then this filter does nothing and the
///   {@link FilterChain} is continued. If it does match then...
/// * An attempt to convert the {@link HttpServletRequest} into an {@link Authentication}
///   is made. If the result is empty, then the filter does nothing more and the
///   {@link FilterChain} is continued. If it does create an {@link Authentication}...
/// * The {@link AuthenticationManager} is used to perform authentication.
/// * If authentication is successful, {@link AuthenticationSuccessHandler} is invoked
///   and the authentication is set on {@link SecurityContextHolder}, else
///   {@link AuthenticationFailureHandler} is invoked
///
/// @author Sergey Bespalov
/// @author Andrey Litvitski

#[derive(Clone)]
pub struct AuthenticationFilter {
    /// Unique per-instance attribute key to ensure this filter runs only once
    /// per request. Mirrors `OncePerRequestFilter.getAlreadyFilteredAttributeName()`.
    already_filtered_attribute_name: String,

    /// The security context holder strategy to use.
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// The request matcher that determines whether this filter should attempt
    /// authentication. Defaults to `AnyRequestMatcher.INSTANCE`.
    request_matcher: Arc<dyn RequestMatcher>,

    /// Converts the `HttpRequest` into an `Authentication`.
    authentication_converter: Arc<dyn AuthenticationConverter>,

    /// Called when authentication is successful.
    success_handler: Arc<dyn AuthenticationSuccessHandler>,

    /// Called when authentication fails.
    failure_handler: Arc<dyn AuthenticationFailureHandler>,

    /// The security context repository used to persist the security context
    /// on authentication success. If not set, the context is only stored on
    /// the `SecurityContextHolderStrategy` for the duration of the request.
    /// Defaults to `None`.
    security_context_repository: Option<Arc<dyn SecurityContextRepository>>,

    /// Resolves an `AuthenticationManager` from the current request.
    authentication_manager_resolver: AuthenticationManagerResolverFn,

    /// Whether Multi-Factor Authentication (MFA) support is enabled.
    mfa_enabled: bool,
}

impl AuthenticationFilter {
    /// The suffix appended to the already-filtered attribute name, matching
    /// Spring Security's `OncePerRequestFilter.ALREADY_FILTERED_SUFFIX`.
    const ALREADY_FILTERED_SUFFIX: &'static str = ".FILTERED";

    /// Creates a new `AuthenticationFilter` with a fixed `AuthenticationManager`.
    ///
    /// # Parameters
    /// * `authentication_manager` - The authentication manager to use.
    /// * `authentication_converter` - Converts the request into an `Authentication`.
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) -> Self {
        let manager = authentication_manager.clone();
        let resolver: AuthenticationManagerResolverFn =
            Arc::new(move |_request: &dyn HttpRequest| manager.clone());
        Self::with_resolver(resolver, authentication_converter)
    }

    /// Creates a new `AuthenticationFilter` with a custom resolver function.
    ///
    /// # Parameters
    /// * `authentication_manager_resolver` - Resolves the `AuthenticationManager`
    ///   from the request context.
    /// * `authentication_converter` - Converts the request into an `Authentication`.
    pub fn with_resolver(
        authentication_manager_resolver: AuthenticationManagerResolverFn,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) -> Self {
        let instance_id = FILTER_INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            already_filtered_attribute_name: format!(
                "{}-{}{}",
                std::any::type_name::<Self>(),
                instance_id,
                Self::ALREADY_FILTERED_SUFFIX
            ),
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            request_matcher: AnyRequestMatcher::instance(),
            authentication_converter,
            success_handler: Arc::new(SavedRequestAwareAuthenticationSuccessHandler::new()),
            failure_handler: Arc::new(SimpleUrlAuthenticationFailureHandler::new("/login")),
            security_context_repository: None,
            authentication_manager_resolver,
            mfa_enabled: false,
        }
    }

    // --- Getters / Setters ---

    /// Returns the request matcher that determines whether this filter
    /// should attempt authentication.
    pub fn get_request_matcher(&self) -> &dyn RequestMatcher {
        self.request_matcher.as_ref()
    }

    /// Sets the request matcher that determines whether this filter
    /// should attempt authentication.
    ///
    /// # Parameters
    /// * `request_matcher` - The request matcher to use. Cannot be null.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Returns the authentication converter.
    pub fn get_authentication_converter(&self) -> &dyn AuthenticationConverter {
        self.authentication_converter.as_ref()
    }

    /// Sets the authentication converter.
    ///
    /// # Parameters
    /// * `authentication_converter` - The converter to use. Cannot be null.
    pub fn set_authentication_converter(
        &mut self,
        authentication_converter: Arc<dyn AuthenticationConverter>,
    ) {
        self.authentication_converter = authentication_converter;
    }

    /// Enables Multi-Factor Authentication (MFA) support.
    ///
    /// # Parameters
    /// * `mfa_enabled` - `true` to enable MFA support, `false` to disable it.
    ///   Default is `false`.
    pub fn set_mfa_enabled(&mut self, mfa_enabled: bool) {
        self.mfa_enabled = mfa_enabled;
    }

    /// Returns the authentication success handler.
    pub fn get_success_handler(&self) -> &dyn AuthenticationSuccessHandler {
        self.success_handler.as_ref()
    }

    /// Sets the authentication success handler.
    ///
    /// # Parameters
    /// * `success_handler` - The handler to invoke on success. Cannot be null.
    pub fn set_success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = success_handler;
    }

    /// Returns the authentication failure handler.
    pub fn get_failure_handler(&self) -> &dyn AuthenticationFailureHandler {
        self.failure_handler.as_ref()
    }

    /// Sets the authentication failure handler.
    ///
    /// # Parameters
    /// * `failure_handler` - The handler to invoke on failure. Cannot be null.
    pub fn set_failure_handler(&mut self, failure_handler: Arc<dyn AuthenticationFailureHandler>) {
        self.failure_handler = failure_handler;
    }

    /// Returns the authentication manager resolver.
    pub fn get_authentication_manager_resolver(&self) -> &AuthenticationManagerResolverFn {
        &self.authentication_manager_resolver
    }

    /// Sets the authentication manager resolver.
    ///
    /// # Parameters
    /// * `authentication_manager_resolver` - The resolver function. Cannot be null.
    pub fn set_authentication_manager_resolver(
        &mut self,
        authentication_manager_resolver: AuthenticationManagerResolverFn,
    ) {
        self.authentication_manager_resolver = authentication_manager_resolver;
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// authentication success. The default action is not to save the
    /// `SecurityContext` (i.e., the repository is `None`).
    ///
    /// # Parameters
    /// * `security_context_repository` - The repository to use. Cannot be null.
    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = Some(security_context_repository);
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use
    /// the strategy stored in `SecurityContextHolder`.
    ///
    /// # Parameters
    /// * `security_context_holder_strategy` - The strategy to use. Cannot be null.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    // --- Internal methods ---

    /// Mirrors `OncePerRequestFilter.getAlreadyFilteredAttributeName()`.
    /// Returns the unique per-instance attribute name used to ensure this filter
    /// is applied only once per request.
    fn get_already_filtered_attribute_name(&self) -> &str {
        &self.already_filtered_attribute_name
    }

    /// Determines whether Multi-Factor Authentication (MFA) authority merging
    /// should be performed.
    ///
    /// Returns `true` when:
    /// * MFA is enabled, and
    /// * The current authentication is not null and is authenticated, and
    /// * The current authentication name matches the new result's name.
    ///
    /// Note: Authority merging depends on the concrete `Authentication`
    /// implementation supporting builder-style mutation. This is not yet
    /// available on the Rust `Authentication` trait, so the flag is checked
    /// but the actual merging is deferred (see TODO in `do_filter`).
    fn should_perform_mfa(
        &self,
        current: Option<&Arc<dyn Authentication>>,
        authentication_result: &dyn Authentication,
    ) -> bool {
        if !self.mfa_enabled {
            return false;
        }
        let Some(current) = current else {
            return false;
        };
        if !current.is_authenticated() {
            return false;
        }
        // Only perform MFA merging if both authentications refer to
        // the same principal.
        current.name() == authentication_result.name()
    }

    /// Handles successful authentication: creates a new security context,
    /// stores the authentication, persists the context (if a repository is
    /// configured), and invokes the success handler.
    async fn successful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Arc<dyn crate::core::Authentication>,
    ) -> Result<(), FilterError> {
        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(authentication.clone()));
        self.security_context_holder_strategy
            .set_context(context.clone());
        if let Some(repo) = &self.security_context_repository {
            repo.save_context(&context, request, response).await;
        }
        self.success_handler
            .on_authentication_success(request, response, authentication.as_ref());
        Ok(())
    }

    /// Handles unsuccessful authentication: clears the security context
    /// and invokes the failure handler.
    fn unsuccessful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        error: AuthenticationError,
    ) -> Result<(), FilterError> {
        self.security_context_holder_strategy.clear_context();
        self.failure_handler
            .on_authentication_failure(request, response, &error)
            .map_err(FilterError::from)
    }

    /// Attempts to authenticate the request:
    /// 1. Converts the request into an `Authentication` via the converter.
    /// 2. Resolves the appropriate `AuthenticationManager`.
    /// 3. Authenticates and returns the result.
    fn attempt_authentication(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Arc<dyn crate::core::Authentication>>, AuthenticationError> {
        let Some(authentication) = self.authentication_converter.convert(request) else {
            return Ok(None);
        };

        let authentication_manager = (self.authentication_manager_resolver)(request);
        let authentication_result = authentication_manager.authenticate(authentication.as_ref())?;

        Ok(Some(authentication_result))
    }
}

#[async_trait]
impl HttpFilter for AuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Ensure the filter is only applied once per request (OncePerRequestFilter).
        let already_filtered_key = self.get_already_filtered_attribute_name();
        if request.get_attribute(already_filtered_key).is_some() {
            return filter_chain.do_filter(request, response).await;
        }
        request.set_attribute(already_filtered_key, AnyValue::Boolean(true));

        // Check if this request matches the configured request matcher.
        if !self.request_matcher.matches(request) {
            if tracing::enabled!(tracing::Level::TRACE) {
                trace!("Did not match request to {:?}", self.request_matcher);
            }
            return filter_chain.do_filter(request, response).await;
        }

        // Attempt authentication conversion and authentication.
        match self.attempt_authentication(request) {
            Ok(Some(authentication_result)) => {
                // Multi-Factor Authentication (MFA) authority merging.
                // In the Java implementation, this uses `Authentication.toBuilder()`
                // to merge the current user's authorities with the new result.
                let current_auth = self
                    .security_context_holder_strategy
                    .get_context()
                    .and_then(|ctx| ctx.get_authentication());

                if self.should_perform_mfa(current_auth.as_deref(), authentication_result.as_ref())
                {
                    // Merge authorities from the current authentication into
                    // the new result. This requires the Authentication implementation
                    // to support builder-style mutation (to_builder).
                    // TODO: Implement authority merging via a to_builder-like
                    // mechanism on the Authentication trait when available.
                    debug!(
                        "MFA authority merging requested for user '{}'",
                        authentication_result.get_name()
                    );
                }

                // Prevent session fixation attacks by changing the session ID.
                // In the Servlet API this is `request.changeSessionId()`.
                // TODO: Add change_session_id() to the HttpRequest/HttpSession trait
                // when session management infrastructure is complete.
                if let Some(session) = request.session() {
                    debug!(
                        "Session exists (id={}); session fixation protection: changeSessionId not yet implemented",
                        session.id()
                    );
                }

                self.successful_authentication(request, response, authentication_result)
                    .await?;
            }
            Ok(None) => {
                // No authentication to perform — converter returned None.
                // Continue the filter chain normally.
                return filter_chain.do_filter(request, response).await;
            }
            Err(ex) => {
                // Authentication failed — invoke the failure handler.
                self.unsuccessful_authentication(request, response, ex)?;
            }
        }

        Ok(())
    }
}

impl Named for AuthenticationFilter {
    fn name(&self) -> &str {
        "AuthenticationFilter"
    }
}
