use next_web_core::{
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse, HttpSession},
};
use std::any::Any;
use std::sync::Arc;
use tracing::{debug, enabled, trace, warn, Level};

use crate::{
    authentication::AuthenticationTrustResolverImpl,
    authorization::AuthenticationTrustResolver,
    core::{
        context::{
            security_context_holder::SecurityContextHolder,
            security_context_holder_strategy::SecurityContextHolderStrategy, SecurityContext,
        },
        Authentication,
    },
    web::context::SecurityContextRepository,
};

/// A `SecurityContextRepository` implementation which stores the security context in
/// the `HttpSession` between requests.
///
/// The `HttpSession` will be queried to retrieve the `SecurityContext` in the
/// `load_context` method (using the key `SPRING_SECURITY_CONTEXT_KEY` by
/// default). If a valid `SecurityContext` cannot be obtained from the
/// `HttpSession` for whatever reason, a fresh `SecurityContext` will be
/// created by calling `SecurityContextHolder::create_empty_context()` and this
/// instance will be returned instead.
///
/// When `save_context` is called, the context will be stored under the same key,
/// provided:
/// 1. The value has changed
/// 2. The configured `AuthenticationTrustResolver` does not report that the
///    contents represent an anonymous user
///
/// With the standard configuration, no `HttpSession` will be created during
/// `load_context` if one does not already exist. When `save_context` is called
/// at the end of the web request, and no session exists, a new `HttpSession` will
/// **only** be created if the supplied `SecurityContext` is not equal to an empty
/// `SecurityContext` instance. This avoids needless `HttpSession` creation,
/// but automates the storage of changes made to the context during the request.
/// Note that if `SecurityContextPersistenceFilter` is configured to eagerly create
/// sessions, then the session-minimisation logic applied here will not make any
/// difference. If you are using eager session creation, then you should ensure that the
/// `allow_session_creation` property of this class is set to `true` (the
/// default).
///
/// If for whatever reason no `HttpSession` should **ever** be created (for
/// example, if Basic authentication is being used or similar clients that will never
/// present the same `jsessionid`), then `set_allow_session_creation(false)`
/// should be set to `false`. Only do this if you really need to conserve server
/// memory and ensure all classes using the `SecurityContextHolder` are designed
/// to have no persistence of the `SecurityContext` between web requests.
pub struct HttpSessionSecurityContextRepository {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// SecurityContext instance used to check for equality with default (unauthenticated) content
    context_object: Arc<dyn SecurityContext>,

    allow_session_creation: bool,
    disable_url_rewriting: bool,
    next_security_context_key: String,
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl HttpSessionSecurityContextRepository {
    /// The default key under which the security context will be stored in the session.
    pub const NEXT_SECURITY_CONTEXT_KEY: &'static str = "NEXT_SECURITY_CONTEXT";

    /// Gets the security context for the current request (if available) and returns it.
    ///
    /// If the session is null, the context object is null or the context object stored in
    /// the session is not an instance of `SecurityContext`, a new context object
    /// will be generated and returned.
    #[deprecated(note = "please see `SecurityContextRepository::load_context`")]
    pub fn load_context(
        &self,
        request_response_holder: &mut HttpRequestResponseHolder,
    ) -> Arc<dyn SecurityContext> {
        let request = request_response_holder.get_request();
        let response = request_response_holder.get_response();
        let http_session = request.get_session(false);

        let mut context = self.read_security_context_from_session(http_session.as_ref());
        if context.is_none() {
            context = Some(self.generate_new_context());
            if enabled!(Level::Trace) {
                trace!("Created {:?}", context);
            }
        }

        let context = context.unwrap();

        if let Some(response) = response {
            let http_session_existed = http_session.is_some();
            let mut wrapped_response = SaveToSessionResponseWrapper::new(
                response,
                request.clone(),
                http_session_existed,
                context.clone(),
            );
            wrapped_response.set_security_context_holder_strategy(
                self.security_context_holder_strategy.clone(),
            );

            request_response_holder.set_response(Arc::new(wrapped_response));
            request_response_holder.set_request(Arc::new(SaveToSessionRequestWrapper::new(
                request.clone(), /* wrapped_response */
            )));
        }

        context
    }

    /// Loads a deferred security context for the given request.
    pub fn load_deferred_context(
        &self,
        request: &dyn HttpServletRequest,
    ) -> Box<dyn DeferredSecurityContext> {
        let spring_security_context_key = self.spring_security_context_key_value.clone();
        let supplier = move || {
            let session = request.get_session(false);
            // We need to capture and return Option here, but Supplier expects SecurityContext
            // This would need adjustment based on your exact trait definitions
            todo!("Implement read_security_context_from_session in supplier")
        };

        Box::new(SupplierDeferredSecurityContext::new(
            supplier,
            self.security_context_holder_strategy.clone(),
        ))
    }

    /// Saves the security context to the HTTP session if necessary.
    pub fn save_context(
        &self,
        context: &dyn SecurityContext,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
    ) {
        // Check if response is a SaveContextOnUpdateOrErrorResponseWrapper
        // This would require downcasting, which depends on your type system
        // Simplified implementation:
        self.save_context_in_http_session(context, request);
    }

    fn save_context_in_http_session(
        &self,
        context: &dyn SecurityContext,
        request: &dyn HttpRequest,
    ) {
        if self.is_transient(context) || self.is_transient(context.get_authentication().as_ref()) {
            return;
        }

        let empty_context = self.generate_new_context();
        if empty_context.equals(context) {
            let session = request.get_session(false);
            self.remove_context_from_session(context, session.as_ref());
        } else {
            let create_session = self.allow_session_creation;
            let session = request.get_session(create_session);
            self.set_context_in_session(context, session.as_ref());
        }
    }

    fn set_context_in_session(
        &self,
        context: &dyn SecurityContext,
        session: Option<&dyn HttpSession>,
    ) {
        if let Some(session) = session {
            session.set_attribute(&self.spring_security_context_key_value, context);
            debug!("Stored {:?} to HttpSession [{:?}]", context, session);
        }
    }

    fn remove_context_from_session(
        &self,
        context: &dyn SecurityContext,
        session: Option<&dyn HttpSession>,
    ) {
        if let Some(session) = session {
            session.remove_attribute(&self.spring_security_context_key_value);
            debug!("Removed {:?} from HttpSession [{:?}]", context, session);
        }
    }

    /// Reads the security context from the HTTP session.
    ///
    /// # Arguments
    ///
    /// * `http_session` - the session obtained from the request.
    fn read_security_context_from_session(
        &self,
        http_session: Option<&dyn HttpSession>,
    ) -> Option<Arc<dyn SecurityContext>> {
        let http_session = http_session?;

        // Session exists, so try to obtain a context from it.
        let context_from_session =
            http_session.get_attribute(&self.spring_security_context_key_value);

        if context_from_session.is_none() {
            trace!(
                "Did not find SecurityContext in HttpSession {} using the SPRING_SECURITY_CONTEXT session attribute",
                http_session.id()
            );
            return None;
        }

        let context_from_session = context_from_session.unwrap();

        // We now have the security context object from the session.
        // Check if it's actually a SecurityContext (would need Any downcast in Rust)
        // For now, assume it's properly typed
        if !self.is_security_context(&context_from_session) {
            warn!(
                "{} did not contain a SecurityContext but contained: '{:?}'; are you improperly \
                 modifying the HttpSession directly (you should always use SecurityContextHolder) \
                 or using the HttpSession attribute reserved for this class?",
                self.spring_security_context_key_value, context_from_session
            );
            return None;
        }

        trace!(
            "Retrieved {:?} from {}",
            context_from_session,
            self.spring_security_context_key_value
        );

        // Everything OK. The only non-null return from this method.
        Some(/* cast to Arc<dyn SecurityContext> */)
    }

    /// By default, calls `SecurityContextHolder::create_empty_context()` to obtain a
    /// new context (there should be no context present in the holder when this method is
    /// called). Using this approach the context creation strategy is decided by the
    /// `SecurityContextHolderStrategy` in use. The default implementations will
    /// return a new `SecurityContextImpl`.
    ///
    /// # Returns
    ///
    /// A new SecurityContext instance. Never null.
    pub fn generate_new_context(&self) -> Arc<dyn SecurityContext> {
        self.security_context_holder_strategy.create_empty_context()
    }

    /// If set to true (the default), a session will be created (if required) to store the
    /// security context if it is determined that its contents are different from the
    /// default empty context value.
    ///
    /// Note that setting this flag to false does not prevent this class from storing the
    /// security context. If your application (or another filter) creates a session, then
    /// the security context will still be stored for an authenticated user.
    ///
    /// # Arguments
    ///
    /// * `allow_session_creation` - whether to allow session creation
    pub fn set_allow_session_creation(&mut self, allow_session_creation: bool) {
        self.allow_session_creation = allow_session_creation;
    }

    /// Allows the use of session identifiers in URLs to be disabled. Off by default.
    ///
    /// # Arguments
    ///
    /// * `disable_url_rewriting` - set to `true` to disable URL encoding methods in
    ///   the response wrapper and prevent the use of `jsessionid` parameters.
    pub fn set_disable_url_rewriting(&mut self, disable_url_rewriting: bool) {
        self.disable_url_rewriting = disable_url_rewriting;
    }

    /// Allows the session attribute name to be customized for this repository instance.
    ///
    /// # Arguments
    ///
    /// * `spring_security_context_key` - the key under which the security context will be
    ///   stored. Defaults to `SPRING_SECURITY_CONTEXT_KEY`.
    pub fn set_spring_security_context_key(&mut self, spring_security_context_key: String) {
        assert!(
            !spring_security_context_key.is_empty(),
            "springSecurityContextKey cannot be empty"
        );
        self.spring_security_context_key_value = spring_security_context_key;
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use
    /// the `SecurityContextHolderStrategy` stored in `SecurityContextHolder`.
    ///
    /// # Arguments
    ///
    /// * `strategy` - the `SecurityContextHolderStrategy` to use
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.context_object = strategy.create_empty_context();
        self.security_context_holder_strategy = strategy;
    }

    fn is_transient(&self, object: Option<&dyn Any>) -> bool {
        match object {
            None => false,
            Some(obj) => {
                // Check for @Transient annotation equivalent
                // This would depend on your annotation/reflection system
                false // Simplified
            }
        }
    }

    /// Sets the `AuthenticationTrustResolver` to be used. The default is
    /// `AuthenticationTrustResolverImpl`.
    ///
    /// # Arguments
    ///
    /// * `trust_resolver` - the `AuthenticationTrustResolver` to use. Cannot be null.
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }

    // Helper method to check if object is a SecurityContext
    fn is_security_context(&self, object: &dyn Any) -> bool {
        // Downcast to SecurityContext would be needed here
        // This depends on your type system implementation
        true // Simplified
    }
}

#[async_trait]
impl SecurityContextRepository for HttpSessionSecurityContextRepository {
    fn load_context(&self, request: &mut dyn HttpRequest) -> Arc<dyn SecurityContext> {
        todo!()
    }

    async fn save_context(
        &self,
        context: &dyn SecurityContext,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) {
        todo!()
    }

    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool {
        let session = request.session();
        match session {
            Some(session) => session.attribute(&self.next_security_context_key).is_some(),
            None => false,
        }
    }
}

impl Default for HttpSessionSecurityContextRepository {
    fn default() -> Self {
        let strategy = SecurityContextHolder::get_context_holder_strategy();
        let empty_context = strategy.create_empty_context();

        Self {
            security_context_holder_strategy: strategy,
            context_object: empty_context,
            allow_session_creation: true,
            disable_url_rewriting: false,
            next_security_context_key: Self::NEXT_SECURITY_CONTEXT_KEY.to_string(),
            trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
        }
    }
}

/// Wrapper that is applied to every request/response to update the
/// `HttpSession` with the `SecurityContext` when a `send_error()` or
/// `send_redirect` happens. See SEC-398.
///
/// Stores the necessary state from the start of the request in order to make a
/// decision about whether the security context has changed before saving it.
struct SaveToSessionRequestWrapper {
    inner: Arc<dyn HttpServletRequestWrapper>,
    response: Arc<SaveContextOnUpdateOrErrorResponseWrapper>,
}

impl SaveToSessionRequestWrapper {
    fn new(
        request: Arc<dyn HttpRequest>,
        response: Arc<SaveContextOnUpdateOrErrorResponseWrapper>,
    ) -> Self {
        Self {
            inner: Arc::new(/* HttpServletRequestWrapper::new(request) */),
            response,
        }
    }

    fn start_async(&self) -> Box<dyn Any> {
        self.response.disable_save_on_response_committed();
        // self.inner.start_async()
        todo!()
    }

    fn start_async_with_request_response(
        &self,
        servlet_request: &dyn Any,
        servlet_response: &dyn Any,
    ) -> Box<dyn Any> {
        self.response.disable_save_on_response_committed();
        // self.inner.start_async(servlet_request, servlet_response)
        todo!()
    }
}

/// Wrapper that is applied to every request/response to update the
/// `HttpSession` with the `SecurityContext` when a `send_error()` or
/// `send_redirect` happens. See SEC-398.
///
/// Stores the necessary state from the start of the request in order to make a
/// decision about whether the security context has changed before saving it.
struct SaveToSessionResponseWrapper {
    inner: SaveContextOnUpdateOrErrorResponseWrapper,
    request: Arc<dyn HttpRequest>,
    http_session_existed_at_start_of_request: bool,
    context_before_execution: Arc<dyn SecurityContext>,
    auth_before_execution: Option<Arc<dyn Authentication>>,
    is_save_context_invoked: bool,
    security_context_holder_strategy: Option<Arc<dyn SecurityContextHolderStrategy>>,
}

impl SaveToSessionResponseWrapper {
    /// Takes the parameters required to call `save_context()` successfully
    /// in addition to the request and the response object we are wrapping.
    ///
    /// # Arguments
    ///
    /// * `response` - the response object we are wrapping
    /// * `request` - the request object (used to obtain the session, if one exists)
    /// * `http_session_existed_at_start_of_request` - indicates whether there was a session
    ///   in place before the filter chain executed. If this is true, and the session is
    ///   found to be null, this indicates that it was invalidated during the request and
    ///   a new session will now be created.
    /// * `context` - the context before the filter chain executed. The context will
    ///   only be stored if it or its contents changed during the request.
    fn new(
        response: Arc<dyn HttpResponse>,
        request: Arc<dyn HttpRequest>,
        http_session_existed_at_start_of_request: bool,
        context: Arc<dyn SecurityContext>,
    ) -> Self {
        let disable_url_rewriting = false; // This would come from the outer repository
        let auth_before_execution = context.get_authentication();

        Self {
            inner: SaveContextOnUpdateOrErrorResponseWrapper::new(response, disable_url_rewriting),
            request,
            http_session_existed_at_start_of_request,
            context_before_execution: context,
            auth_before_execution,
            is_save_context_invoked: false,
            security_context_holder_strategy: None,
        }
    }

    fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = Some(strategy);
    }

    /// Stores the supplied security context in the session (if available) and if it
    /// has changed since it was set at the start of the request. If the
    /// AuthenticationTrustResolver identifies the current user as anonymous, then the
    /// context will not be stored.
    ///
    /// # Arguments
    ///
    /// * `context` - the context object obtained from the SecurityContextHolder after
    ///   the request has been processed by the filter chain.
    ///   SecurityContextHolder.get_context() cannot be used to obtain the context as it
    ///   has already been cleared by the time this method is called.
    fn save_context(&mut self, context: Arc<dyn SecurityContext>) {
        // Check if transient
        if self.is_transient(Some(&context)) {
            return;
        }

        let authentication = context.get_authentication();

        if self.is_transient(authentication.as_ref().map(|a| a.as_ref() as &dyn Any)) {
            return;
        }

        let mut http_session = self.request.get_session(false);
        let spring_security_context_key = "SPRING_SECURITY_CONTEXT"; // Should come from outer repository

        // See SEC-776
        if authentication.is_none() || self.is_anonymous(authentication.as_ref()) {
            if http_session.is_some() && self.auth_before_execution.is_some() {
                // SEC-1587 A non-anonymous context may still be in the session
                // SEC-1735 remove if the contextBeforeExecution was not anonymous
                if let Some(ref session) = http_session {
                    session.remove_attribute(spring_security_context_key);
                }
                self.is_save_context_invoked = true;
            }

            debug!(
                "Did not store {} SecurityContext",
                if authentication.is_none() {
                    "empty"
                } else {
                    "anonymous"
                }
            );
            return;
        }

        // If no session exists, try to create one if allowed
        if http_session.is_none() {
            http_session = self.create_new_session_if_allowed(&context);
        }

        // If HttpSession exists, store current SecurityContext but only if it has
        // actually changed in this thread (see SEC-37, SEC-1307, SEC-1528)
        if let Some(ref session) = http_session {
            // We may have a new session, so check also whether the context attribute is set SEC-1561
            if self.context_changed(&context)
                || session.get_attribute(spring_security_context_key).is_none()
            {
                // HttpSessionSecurityContextRepository.saveContextInHttpSession(context, request)
                self.is_save_context_invoked = true;
            }
        }
    }

    fn context_changed(&self, context: &Arc<dyn SecurityContext>) -> bool {
        self.is_save_context_invoked
            || !Arc::ptr_eq(context, &self.context_before_execution)
            || context.get_authentication().as_ref().map(|a| a.as_ref())
                != self.auth_before_execution.as_ref().map(|a| a.as_ref())
    }

    fn create_new_session_if_allowed(
        &self,
        context: &Arc<dyn SecurityContext>,
    ) -> Option<Arc<dyn HttpSession>> {
        if self.http_session_existed_at_start_of_request {
            debug!(
                "HttpSession is now null, but was not null at start of request; \
                 session was invalidated, so do not create a new session"
            );
            return None;
        }

        if !self.allow_session_creation() {
            debug!(
                "The HttpSession is currently null, and the HttpSessionSecurityContextRepository \
                 is prohibited from creating an HttpSession (because the allowSessionCreation property \
                 is false) - SecurityContext thus not stored for next request"
            );
            return None;
        }

        // Generate a HttpSession only if we need to
        if self.is_empty_context(context) {
            debug!(
                "HttpSession is null, but SecurityContext has not changed from default empty \
                 context {:?} so not creating HttpSession or storing SecurityContext",
                context
            );
            return None;
        }

        match self.request.get_session(true) {
            Some(session) => {
                debug!("Created HttpSession as SecurityContext is non-default");
                Some(session)
            }
            None => {
                // Response must already be committed, therefore can't create a new session
                warn!(
                    "Failed to create a session, as response has been committed. \
                     Unable to store SecurityContext."
                );
                None
            }
        }
    }

    fn is_transient(&self, object: Option<&dyn Any>) -> bool {
        match object {
            None => false,
            Some(_obj) => {
                // Check for @Transient annotation equivalent
                false // Simplified
            }
        }
    }

    fn is_anonymous(&self, authentication: Option<&Arc<dyn Authentication>>) -> bool {
        match authentication {
            Some(auth) => {
                // Use trust_resolver to check if anonymous
                // self.trust_resolver.is_anonymous(auth.as_ref())
                false // Simplified
            }
            None => true,
        }
    }

    fn allow_session_creation(&self) -> bool {
        // This should reference the outer repository's setting
        true // Simplified
    }

    fn is_empty_context(&self, context: &Arc<dyn SecurityContext>) -> bool {
        // Compare with empty context from the repository
        false // Simplified
    }
}
