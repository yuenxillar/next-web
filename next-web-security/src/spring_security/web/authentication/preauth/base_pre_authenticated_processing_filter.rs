use std::{any::TypeId, sync::Arc};

use next_web_context::ApplicationEventPublisher;
use next_web_core::{
    anys::any_value::AnyValue,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::HttpFilterChain,
        http::{http_request::HttpRequest, http_response::HttpResponse},
    },
};
use tracing::{debug, trace};

use crate::{
    authentication::{
        authentication_details_source::AuthenticationDetailsSource,
        event::InteractiveAuthenticationSuccessEvent,
    },
    authorization::AuthenticationManager,
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication, AuthenticationError,
    },
    web::{
        authentication::{
            AuthPrincipal, AuthenticationFailureHandler, AuthenticationSuccessHandler,
            WebAuthenticationDetailsSource,
        },
        context::{HttpSessionSecurityContextRepository, SecurityContextRepository},
        util::matcher::RequestMatcher,
        WebAttributes,
    },
};

use super::pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken;

/// Extension implemented by concrete pre-authenticated processing filters.
///
/// It corresponds to the abstract `getPreAuthenticatedPrincipal` and
/// `getPreAuthenticatedCredentials` methods of the base filter. Subclasses must extract
/// the necessary information on the principal from the incoming request, rather than
/// authenticate them. It is assumed that the external system is responsible for the
/// accuracy of the data and preventing the submission of forged values.
pub trait BasePreAuthenticatedProcessingFilterExt {
    /// Extracts the pre-authenticated principal from the current request.
    fn get_pre_authenticated_principal(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError>;

    /// Extracts the pre-authenticated credentials (if applicable) from the current
    /// request. Should not return `None` for a valid principal, though some
    /// implementations may return a dummy value.
    fn get_pre_authenticated_credentials(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError>;

    /// Determines if the current principal has changed.
    ///
    /// The default implementation compares the pre-authenticated principal against the
    /// authentication name and, when available, against the authentication principal
    /// itself. Subclasses can override this method to determine when a principal has
    /// changed.
    fn principal_changed(
        &self,
        request: &dyn HttpRequest,
        current_authentication: &dyn Authentication,
    ) -> Result<bool, AuthenticationError> {
        let principal = self.get_pre_authenticated_principal(request)?;
        let Some(principal) = principal else {
            return Ok(true);
        };

        let principal = principal.to_string();
        if current_authentication
            .name()
            .eq_ignore_ascii_case(&principal)
        {
            return Ok(false);
        }
        if let Some(current_principal) = current_authentication.principal() {
            if current_principal.to_string() == principal {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Base class for processing filters that handle pre-authenticated authentication
/// requests, where it is assumed that the principal has already been authenticated by an
/// external system.
///
/// The purpose is then only to extract the necessary information on the principal from
/// the incoming request, rather than to authenticate them. External authentication
/// systems may provide this information via request data such as headers or cookies which
/// the pre-authentication system can extract. It is assumed that the external system is
/// responsible for the accuracy of the data and preventing the submission of forged
/// values.
///
/// If the security context already contains an `Authentication` object, the filter will
/// do nothing by default. You can force it to check for a change in the principal by
/// setting [`Self::set_check_for_principal_changes`].
///
/// By default, the filter chain will proceed when an authentication attempt fails in
/// order to allow other authentication mechanisms to process the request. To reject the
/// credentials immediately, set
/// [`Self::set_continue_filter_chain_on_unsuccessful_authentication`] to `false`.
///
/// The filter saves the `SecurityContext` using the configured
/// `SecurityContextRepository`.
#[derive(Clone)]
pub struct BasePreAuthenticatedProcessingFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    event_publisher: Option<Arc<dyn ApplicationEventPublisher>>,
    authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    continue_filter_chain_on_unsuccessful_authentication: bool,
    check_for_principal_changes: bool,
    invalidate_session_on_principal_change: bool,
    authentication_success_handler: Option<Arc<dyn AuthenticationSuccessHandler>>,
    authentication_failure_handler: Option<Arc<dyn AuthenticationFailureHandler>>,
    requires_authentication_request_matcher: Option<Arc<dyn RequestMatcher>>,
    security_context_repository: Arc<dyn SecurityContextRepository>,
    mfa_enabled: bool,
}

impl BasePreAuthenticatedProcessingFilter {
    /// Creates a new instance using the supplied authentication manager.
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager: Some(authentication_manager),
            ..Default::default()
        }
    }

    /// Checks that all required properties have been set.
    pub fn after_properties_set(&self) {
        debug_assert!(
            self.authentication_manager.is_some(),
            "An AuthenticationManager must be set"
        );
    }

    /// Sets the `ApplicationEventPublisher` to use.
    pub fn set_application_event_publisher(
        &mut self,
        event_publisher: Arc<dyn ApplicationEventPublisher>,
    ) {
        self.event_publisher = Some(event_publisher);
    }

    /// Enables Multi-Factor Authentication support. Defaults to `false`.
    pub fn set_mfa_enabled(&mut self, mfa_enabled: bool) {
        self.mfa_enabled = mfa_enabled;
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// authentication success.
    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = security_context_repository;
    }

    /// Sets the `AuthenticationDetailsSource` to use.
    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = authentication_details_source;
    }

    /// Returns the `AuthenticationDetailsSource` in use.
    pub fn get_authentication_details_source(&self) -> &Arc<dyn AuthenticationDetailsSource> {
        &self.authentication_details_source
    }

    /// Sets the `AuthenticationManager` to use.
    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = Some(authentication_manager);
    }

    /// Returns the `AuthenticationManager` in use, if any.
    pub fn get_authentication_manager(&self) -> Option<&Arc<dyn AuthenticationManager>> {
        self.authentication_manager.as_ref()
    }

    /// If set to `true` (the default), any authentication error raised by the
    /// authentication manager is swallowed and the request is allowed to proceed.
    pub fn set_continue_filter_chain_on_unsuccessful_authentication(
        &mut self,
        should_continue: bool,
    ) {
        self.continue_filter_chain_on_unsuccessful_authentication = should_continue;
    }

    /// If set, the pre-authenticated principal is checked on each request and compared
    /// against the current authentication. If a change is detected, the user is
    /// reauthenticated.
    pub fn set_check_for_principal_changes(&mut self, check_for_principal_changes: bool) {
        self.check_for_principal_changes = check_for_principal_changes;
    }

    /// Returns whether principal changes are checked on each request.
    pub fn is_check_for_principal_changes(&self) -> bool {
        self.check_for_principal_changes
    }

    /// If a change of principal is detected, determines whether any existing session
    /// should be invalidated before proceeding to authenticate the new principal.
    /// Defaults to `true`.
    pub fn set_invalidate_session_on_principal_change(
        &mut self,
        invalidate_session_on_principal_change: bool,
    ) {
        self.invalidate_session_on_principal_change = invalidate_session_on_principal_change;
    }

    /// Returns whether the existing session is invalidated when the principal changes.
    pub fn is_invalidate_session_on_principal_change(&self) -> bool {
        self.invalidate_session_on_principal_change
    }

    /// Sets the strategy used to handle a successful authentication.
    pub fn set_authentication_success_handler(
        &mut self,
        authentication_success_handler: Arc<dyn AuthenticationSuccessHandler>,
    ) {
        self.authentication_success_handler = Some(authentication_success_handler);
    }

    /// Sets the strategy used to handle a failed authentication.
    pub fn set_authentication_failure_handler(
        &mut self,
        authentication_failure_handler: Arc<dyn AuthenticationFailureHandler>,
    ) {
        self.authentication_failure_handler = Some(authentication_failure_handler);
    }

    /// Sets the request matcher to check whether to proceed the request further.
    pub fn set_requires_authentication_request_matcher(
        &mut self,
        requires_authentication_request_matcher: Arc<dyn RequestMatcher>,
    ) {
        self.requires_authentication_request_matcher =
            Some(requires_authentication_request_matcher);
    }

    /// Returns the configured request matcher, if any.
    pub fn get_requires_authentication_request_matcher(&self) -> Option<&Arc<dyn RequestMatcher>> {
        self.requires_authentication_request_matcher.as_ref()
    }

    /// Sets the `SecurityContextHolderStrategy` to use.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    /// Returns the `SecurityContextHolderStrategy` in use.
    pub fn get_security_context_holder_strategy(&self) -> &Arc<dyn SecurityContextHolderStrategy> {
        &self.security_context_holder_strategy
    }

    /// Returns the current authentication, if any.
    pub fn get_current_authentication(&self) -> Option<Arc<dyn Authentication>> {
        self.security_context_holder_strategy
            .get_context()
            .get_authentication()
    }

    /// Clears the current security context.
    pub fn clear_context(&self) {
        self.security_context_holder_strategy.clear_context();
    }

    /// Tries to authenticate a pre-authenticated user if the user has not yet been
    /// authenticated.
    ///
    /// This is the equivalent of the base filter's `doFilter` method: when the request
    /// matches the configured request matcher the pre-authenticated user is
    /// authenticated, otherwise the request is passed to the filter chain unchanged.
    pub async fn do_filter<P>(
        &self,
        support: &P,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError>
    where
        P: BasePreAuthenticatedProcessingFilterExt,
    {
        if self
            .requires_authentication(support, request)
            .map_err(FilterError::from)?
        {
            debug!(
                "Authenticating {:?}",
                self.get_current_authentication()
                    .map(|auth| auth.name().into_owned())
            );
            self.do_authenticate(support, request, response).await?;
        } else {
            trace!("Did not authenticate since request did not match");
        }

        filter_chain.do_filter(request, response).await
    }

    /// Request matcher for the default auth check logic.
    fn requires_authentication<P>(
        &self,
        support: &P,
        request: &mut dyn HttpRequest,
    ) -> Result<bool, AuthenticationError>
    where
        P: BasePreAuthenticatedProcessingFilterExt,
    {
        if let Some(request_matcher) = self.requires_authentication_request_matcher.as_ref() {
            return Ok(request_matcher.matches(request));
        }

        let Some(current_user) = self.get_current_authentication() else {
            return Ok(true);
        };
        if !self.check_for_principal_changes {
            return Ok(false);
        }
        if !support.principal_changed(request, current_user.as_ref())? {
            return Ok(false);
        }

        debug!("Pre-authenticated principal has changed and will be reauthenticated");

        if self.invalidate_session_on_principal_change {
            self.security_context_holder_strategy.clear_context();
            if let Some(session) = request.session_mut(false) {
                debug!("Invalidating existing session");
                session.invalidate();
            }
        }

        Ok(true)
    }

    /// Does the actual authentication for a pre-authenticated user.
    async fn do_authenticate<P>(
        &self,
        support: &P,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError>
    where
        P: BasePreAuthenticatedProcessingFilterExt,
    {
        let Some(principal) = support
            .get_pre_authenticated_principal(request)
            .map_err(FilterError::from)?
        else {
            debug!("No pre-authenticated principal found in request");
            return Ok(());
        };

        debug!(
            "preAuthenticatedPrincipal = {}, trying to authenticate",
            principal
        );

        let credentials = support
            .get_pre_authenticated_credentials(request)
            .map_err(FilterError::from)?;

        let mut authentication_request =
            PreAuthenticatedAuthenticationToken::new(principal, credentials);
        authentication_request.set_details(Some(
            self.authentication_details_source.build_details(request),
        ));

        let Some(authentication_manager) = self.authentication_manager.as_ref() else {
            return Err(FilterError::custom("An AuthenticationManager must be set"));
        };

        match authentication_manager
            .authenticate(&authentication_request)
            .await
        {
            Ok(authentication_result) => {
                let current = self.get_current_authentication();
                let authentication_result = if self
                    .should_perform_mfa(current.as_deref(), authentication_result.as_ref())
                {
                    self.apply_mfa(authentication_result, current)
                } else {
                    authentication_result
                };

                self.successful_authentication(request, response, authentication_result)
                    .await?;
            }
            Err(error) => {
                self.unsuccessful_authentication(request, response, &error)?;
                if !self.continue_filter_chain_on_unsuccessful_authentication {
                    return Err(FilterError::from(error));
                }
            }
        }

        Ok(())
    }

    /// Determines whether the current authorities should be merged into the
    /// authentication result (multi-factor authentication support).
    fn should_perform_mfa(
        &self,
        current: Option<&dyn Authentication>,
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
        current.name() == authentication_result.name()
    }

    /// Merges the authorities of the current authentication into the authentication
    /// result.
    fn apply_mfa(
        &self,
        authentication_result: Arc<dyn Authentication>,
        current: Option<Arc<dyn Authentication>>,
    ) -> Arc<dyn Authentication> {
        let Some(current) = current else {
            return authentication_result;
        };

        let mut builder = authentication_result.to_builder();
        builder.authorities(Box::new(move |authorities| {
            for current_authority in current.authorities() {
                let already_present = authorities
                    .iter()
                    .any(|authority| authority.authority() == current_authority.authority());
                if !already_present {
                    authorities.push(current_authority.clone());
                }
            }
        }));
        builder.build()
    }

    /// Puts the `Authentication` instance returned by the authentication manager into the
    /// security context.
    async fn successful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        auth_result: Arc<dyn Authentication>,
    ) -> Result<(), BoxError> {
        debug!("Authentication success: {}", auth_result.name());

        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(auth_result.clone()));
        self.security_context_holder_strategy
            .set_context(context.clone());
        self.security_context_repository
            .save_context(&context, request, response)
            .await;

        if let Some(event_publisher) = self.event_publisher.as_ref() {
            event_publisher
                .publish_event(Box::new(InteractiveAuthenticationSuccessEvent::new(
                    auth_result.clone(),
                    TypeId::of::<Self>(),
                )))
                .ok();
        }

        if let Some(authentication_success_handler) = self.authentication_success_handler.as_ref() {
            authentication_success_handler.on_authentication_success(
                request,
                response,
                auth_result.as_ref(),
            );
        }

        Ok(())
    }

    /// Ensures the authentication object in the security context is set to null when
    /// authentication fails, and caches the failure exception as a request attribute.
    fn unsuccessful_authentication(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        failed: &AuthenticationError,
    ) -> Result<(), BoxError> {
        self.security_context_holder_strategy.clear_context();
        debug!("Cleared security context due to exception: {}", failed);

        request.set_attribute(
            WebAttributes::AUTHENTICATION_ERROR,
            AnyValue::Object(Box::new(failed.clone())),
        );

        if let Some(authentication_failure_handler) = self.authentication_failure_handler.as_ref() {
            authentication_failure_handler.on_authentication_failure(request, response, failed)?;
        }

        Ok(())
    }
}

impl Default for BasePreAuthenticatedProcessingFilter {
    fn default() -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            event_publisher: None,
            authentication_details_source: Arc::new(WebAuthenticationDetailsSource::default()),
            authentication_manager: None,
            continue_filter_chain_on_unsuccessful_authentication: true,
            check_for_principal_changes: false,
            invalidate_session_on_principal_change: true,
            authentication_success_handler: None,
            authentication_failure_handler: None,
            requires_authentication_request_matcher: None,
            security_context_repository: Arc::new(HttpSessionSecurityContextRepository::default()),
            mfa_enabled: false,
        }
    }
}
