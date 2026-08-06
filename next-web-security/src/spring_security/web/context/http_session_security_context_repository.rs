use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse, HttpSession},
};
use std::sync::Arc;
use tracing::{debug, enabled, trace, warn, Level};

use crate::{
    authentication::AuthenticationTrustResolverImpl,
    authorization::AuthenticationTrustResolver,
    core::context::{
        DeferredSecurityContext, SecurityContext, SecurityContextHolder,
        SecurityContextHolderStrategy,
    },
    web::context::{SecurityContextRepository, SuppliedDeferredSecurityContext},
};

/// A `SecurityContextRepository` implementation which stores the security context in
/// the `HttpSession` between requests.
///
/// The `HttpSession` will be queried to retrieve the `SecurityContext` in the
/// `load_context` method (using the key `NEXT_SECURITY_CONTEXT_KEY` by
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

    fn save_context_in_http_session(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
    ) {
        if context.get_authentication().is_none() {
            return;
        }

        let empty_context = self.generate_new_context();
        if Arc::ptr_eq(&empty_context, context) {
            let session = request.session();
            self.remove_context_from_session(context.as_ref(), session);
        } else {
            let session = request.session_mut(self.allow_session_creation);
            self.set_context_in_session(context, session);
        }
    }

    fn set_context_in_session(
        &self,
        context: &Arc<dyn SecurityContext>,
        session: Option<&mut dyn HttpSession>,
    ) {
        if let Some(session) = session {
            session.set_attribute(
                &self.next_security_context_key,
                AnyValue::Object(Box::new(context.to_owned())),
            );
            debug!(
                "Stored SecurityContext to HttpSession [{}]",
                session.to_string()
            );
        }
    }

    fn remove_context_from_session(
        &self,
        _context: &dyn SecurityContext,
        session: Option<&dyn HttpSession>,
    ) {
        if let Some(session) = session {
            session.remove_attribute(&self.next_security_context_key);
            debug!(
                "Removed SecurityContext from HttpSession [{}]",
                session.id()
            );
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
        let http_session = if let Some(http_session) = http_session {
            trace!("No HttpSession currently exists");
            http_session
        } else {
            return None;
        };

        // Session exists, so try to obtain a context from it.
        let context_from_session = match http_session.attribute(&self.next_security_context_key) {
            Some(ctx) => ctx,
            None => {
                trace!(
                   "Did not find SecurityContext in HttpSession {} using the NEXT_SECURITY_CONTEXT session attribute",
                   http_session.id()
               );
                return None;
            }
        };

        // We now have the security context object from the session.
        // Check if it's actually a SecurityContext (would need Any downcast in Rust)
        // For now, assume it's properly typed
        let context_from_session = match context_from_session
            .as_ref_object::<Arc<dyn SecurityContext>>()
        {
            Some(s) => s,
            None => {
                warn!(
                    "{} did not contain a SecurityContext but contained: '{:?}'; are you improperly \
                     modifying the HttpSession directly (you should always use SecurityContextHolder) \
                     or using the HttpSession attribute reserved for this class?",
                    self.next_security_context_key, context_from_session
                );
                return None;
            }
        };

        if enabled!(Level::TRACE) {
            trace!(
                "Retrieved ContextFromSession from {}",
                self.next_security_context_key
            );
        }

        Some(context_from_session.to_owned())
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
    /// * `next_security_context_key` - the key under which the security context will be
    ///   stored. Defaults to `NEXT_SECURITY_CONTEXT_KEY`.
    pub fn set_next_security_context_key(&mut self, next_security_context_key: impl Into<String>) {
        let next_security_context_key = next_security_context_key.into();
        assert!(
            !next_security_context_key.is_empty(),
            "NextSecurityContextKey cannot be empty"
        );
        self.next_security_context_key = next_security_context_key.into();
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

    /// Sets the `AuthenticationTrustResolver` to be used. The default is
    /// `AuthenticationTrustResolverImpl`.
    ///
    /// # Arguments
    ///
    /// * `trust_resolver` - the `AuthenticationTrustResolver` to use. Cannot be null.
    pub fn set_trust_resolver(&mut self, trust_resolver: Arc<dyn AuthenticationTrustResolver>) {
        self.trust_resolver = trust_resolver;
    }
}

#[async_trait]
impl SecurityContextRepository for HttpSessionSecurityContextRepository {
    fn load_deferred_context(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Box<dyn DeferredSecurityContext> {
        let security_context = self.read_security_context_from_session(request.session());

        Box::new(SuppliedDeferredSecurityContext::new(
            security_context,
            self.security_context_holder_strategy.to_owned(),
        ))
    }

    async fn save_context(
        &self,
        context: &Arc<dyn SecurityContext>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
        self.save_context_in_http_session(context, request);
    }

    fn contains_context(&self, request: &mut dyn HttpRequest) -> bool {
        match request.session() {
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
