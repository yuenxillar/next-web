//! Configures the web security of the application.
//!
//! [`WebSecurity`] collects the builders that create the
//! [`SecurityFilterChain`]s of the application, the requests security should
//! ignore, and the firewall the filter chain proxy validates the requests with,
//! and builds the [`FilterChainProxy`] the web layer installs in front of the
//! routes.
//!
//! The type is a [`SecurityBuilder`] of its own, so it runs the configurers that
//! were registered for it before it builds the filter chain proxy, exactly like
//! [`HttpSecurity`](crate::config::web::builders::HttpSecurity) builds one
//! filter chain.

use std::{
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Arc},
};

use tracing::warn;

use crate::{
    config::{
        base_configured_security_builder::{
            BaseConfiguredSecurityBuilder, BaseConfiguredSecurityBuilderExt, BuildState,
        },
        security_builder::SecurityBuilder,
        web::BaseRequestMatcherRegistry,
    },
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        firewall::{request_rejected_handler::RequestRejectedHandler, HttpFirewall},
        security_filter_chain::SecurityFilterChain,
        util::matcher::PathPatternRequestMatcherBuilder,
        FilterChainProxy, FilterChainValidator,
    },
};

/// The web security of the application.
///
/// A configuration of the application creates an instance of this type, adds
/// the builders of the security filter chains, optionally ignores a few static
/// requests, and builds the [`FilterChainProxy`] from it. The proxy is then
/// registered as an [`HttpFilter`](next_web_core::traits::filter::HttpFilter),
/// so that the filter layer of the application runs it in front of the routes.
///
/// # Examples
///
/// ```ignore
/// let mut web_security = WebSecurity::default();
/// web_security
///     .ignoring()
///     .request_matchers(vec!["/resources/**", "/static/**"]);
/// web_security.add_security_filter_chain_builder(http_security);
///
/// let filter_chain_proxy = web_security.build();
/// ```
pub struct WebSecurity {
    /// The builders that create one [`SecurityFilterChain`] each.
    security_filter_chain_builders: Vec<Box<dyn SecurityBuilder<DefaultSecurityFilterChain>>>,
    /// The security filter chains that are already built.
    security_filter_chains: Vec<Arc<dyn SecurityFilterChain>>,
    /// The registry of the requests security should ignore.
    ignored_request_registry: IgnoredRequestConfigurer,
    /// The firewall the built proxy validates the requests with.
    http_firewall: Option<Arc<dyn HttpFirewall>>,
    /// The handler of a request the firewall rejects.
    request_rejected_handler: Option<Arc<dyn RequestRejectedHandler>>,
    /// Whether the debug support reports the requests the proxy handles.
    debug_enabled: bool,
    /// The action that runs after the proxy has been built.
    post_build_action: Option<Box<dyn FnMut() + Send + Sync>>,
    /// The configurers and the shared objects of the builder.
    base: BaseConfiguredSecurityBuilder<FilterChainProxy, Self>,
}

impl WebSecurity {
    /// Creates an instance without any security filter chain builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            security_filter_chain_builders: Vec::new(),
            security_filter_chains: Vec::new(),
            ignored_request_registry: IgnoredRequestConfigurer::new(),
            http_firewall: None,
            request_rejected_handler: None,
            debug_enabled: false,
            post_build_action: None,
            base: Default::default(),
        }
    }

    /// Returns the configurer that registers the requests security should
    /// ignore.
    ///
    /// The security of the application, including the security context, is not
    /// available on a request that matches one of the registered matchers. Only
    /// static resources should be registered, because a dynamic request should
    /// be mapped to allow all users by the authorization rules instead.
    ///
    /// The following example ignores every URL that starts with `/resources/`
    /// or `/static/`:
    ///
    /// ```ignore
    /// web_security
    ///     .ignoring()
    ///     .request_matchers(vec!["/resources/**", "/static/**"]);
    /// ```
    ///
    /// Every call is additive: the matchers of a later call are ignored as
    /// well.
    pub fn ignoring(&mut self) -> &mut IgnoredRequestConfigurer {
        &mut self.ignored_request_registry
    }

    /// Sets the firewall the built filter chain proxy validates the requests
    /// with.
    ///
    /// The default is a strict firewall, which rejects suspicious requests.
    ///
    /// # Arguments
    ///
    /// * `http_firewall` - The firewall to use.
    pub fn http_firewall(&mut self, http_firewall: Arc<dyn HttpFirewall>) -> &mut Self {
        self.http_firewall = Some(http_firewall);
        self
    }

    /// Controls the debug support of the web security.
    ///
    /// The debug support is disabled by default. It reports information about
    /// the requests the filter chain proxy handles, which can include sensitive
    /// information, so it must not be enabled in a production system.
    ///
    /// # Arguments
    ///
    /// * `debug_enabled` - Whether the debug support is enabled.
    pub fn debug(&mut self, debug_enabled: bool) -> &mut Self {
        self.debug_enabled = debug_enabled;
        self
    }

    /// Adds the builder that creates one [`SecurityFilterChain`].
    ///
    /// The framework adds the builder of every declared filter chain while the
    /// application starts, so this method is normally called for the
    /// application.
    ///
    /// # Arguments
    ///
    /// * `security_filter_chain_builder` - The builder of the filter chain.
    pub fn add_security_filter_chain_builder<B>(
        &mut self,
        security_filter_chain_builder: B,
    ) -> &mut Self
    where
        B: SecurityBuilder<DefaultSecurityFilterChain> + 'static,
    {
        self.security_filter_chain_builders
            .push(Box::new(security_filter_chain_builder));
        self
    }

    /// Adds a security filter chain that is already built.
    ///
    /// A filter chain an application declares is added through this method,
    /// while a builder such as an HTTP security configuration is added through
    /// [`Self::add_security_filter_chain_builder`].
    ///
    /// # Arguments
    ///
    /// * `security_filter_chain` - The filter chain to add.
    pub fn add_security_filter_chain(
        &mut self,
        security_filter_chain: Arc<dyn SecurityFilterChain>,
    ) -> &mut Self {
        self.security_filter_chains.push(security_filter_chain);
        self
    }

    /// Sets the action that runs immediately after the filter chain proxy has
    /// been built.
    ///
    /// # Arguments
    ///
    /// * `post_build_action` - The action to run.
    pub fn post_build_action<F>(&mut self, post_build_action: F) -> &mut Self
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.post_build_action = Some(Box::new(post_build_action));
        self
    }

    /// Sets the handler of the requests the firewall rejects.
    ///
    /// The default is a handler that answers with a bad request status.
    ///
    /// # Arguments
    ///
    /// * `request_rejected_handler` - The handler to use.
    pub fn request_rejected_handler(
        &mut self,
        request_rejected_handler: Arc<dyn RequestRejectedHandler>,
    ) -> &mut Self {
        self.request_rejected_handler = Some(request_rejected_handler);
        self
    }
}

impl Default for WebSecurity {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityBuilder<FilterChainProxy> for WebSecurity {
    /// Builds the filter chain proxy the web layer installs.
    ///
    /// The configurers of the builder are initialized and configured before the
    /// proxy is built, so a configurer can still contribute a filter chain
    /// builder, a firewall, or an ignored request.
    fn build(&mut self) -> FilterChainProxy {
        assert!(
            !self
                .base
                .base_security_builder
                .building
                .swap(true, Ordering::SeqCst),
            "This object has already been built"
        );

        self.base.build_state = BuildState::INITIALIZING;
        self.before_init();
        BaseConfiguredSecurityBuilder::init_configurers(self);
        self.base.build_state = BuildState::CONFIGURING;
        self.before_configure();
        BaseConfiguredSecurityBuilder::configure_configurers(self);
        self.base.build_state = BuildState::BUILDING;
        let result = self.perform_build();
        self.base.build_state = BuildState::BUILT;

        result
    }
}

impl BaseConfiguredSecurityBuilderExt<FilterChainProxy, Self> for WebSecurity {
    fn perform_build(&mut self) -> FilterChainProxy {
        assert!(
            !self.security_filter_chain_builders.is_empty()
                || !self.security_filter_chains.is_empty(),
            "At least one SecurityBuilder<DefaultSecurityFilterChain> or security filter chain \
             needs to be specified. Typically this is done by exposing a filter chain, and more \
             advanced users can call WebSecurity::add_security_filter_chain_builder or \
             WebSecurity::add_security_filter_chain directly"
        );

        let ignored_requests = self.ignored_request_registry.take_request_matchers();
        let mut security_filter_chains: Vec<Arc<dyn SecurityFilterChain>> = Vec::with_capacity(
            ignored_requests.len()
                + self.security_filter_chains.len()
                + self.security_filter_chain_builders.len(),
        );

        // An ignored request is a filter chain without filters, which matches
        // the request and leaves it unsecured.
        for ignored_request in ignored_requests {
            warn!(
                "The request matcher {:?} is ignored by the security filter chains. This is not \
                 recommended; map the request to allow all users with the authorization rules of \
                 the HTTP security instead.",
                ignored_request
            );

            security_filter_chains.push(Arc::new(DefaultSecurityFilterChain::new(
                ignored_request,
                Vec::new(),
            )));
        }

        // The filter chains the application declared follow the ignored
        // requests, and the chains of the builders follow them.
        security_filter_chains.extend(self.security_filter_chains.iter().cloned());

        for builder in &mut self.security_filter_chain_builders {
            let security_filter_chain = builder.build();
            security_filter_chains.push(Arc::new(security_filter_chain));
        }

        let mut filter_chain_proxy = FilterChainProxy::new(security_filter_chains);

        if let Some(http_firewall) = self.http_firewall.as_ref() {
            filter_chain_proxy.set_firewall(http_firewall.clone());
        }

        if let Some(request_rejected_handler) = self.request_rejected_handler.as_ref() {
            filter_chain_proxy.set_request_rejected_handler(request_rejected_handler.clone());
        }

        filter_chain_proxy.set_filter_chain_validator(Arc::new(WebSecurityFilterChainValidator));
        filter_chain_proxy.after_properties_set();

        if self.debug_enabled {
            // The debug support reports the request and the response of every
            // request, which the tracing layer of the application does as well,
            // so only the warning of the setting is reported here.
            warn!(
                "Security debugging is enabled. The debug information may include sensitive \
                 information, so it must not be used in a production system."
            );
        }

        if let Some(post_build_action) = self.post_build_action.as_mut() {
            post_build_action();
        }

        filter_chain_proxy
    }
}

impl Deref for WebSecurity {
    type Target = BaseConfiguredSecurityBuilder<FilterChainProxy, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for WebSecurity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Registers the requests the security filter chains ignore.
///
/// The configurer is reached through [`WebSecurity::ignoring`], and every
/// matcher it registers is added to the filter chain proxy as a filter chain
/// without filters, which matches the request and leaves it unsecured.
#[derive(Clone)]
pub struct IgnoredRequestConfigurer {
    /// The registry that creates and collects the matchers.
    base: BaseRequestMatcherRegistry<Self>,
}

impl IgnoredRequestConfigurer {
    /// Creates an empty configurer.
    fn new() -> Self {
        let mut base = BaseRequestMatcherRegistry::<Self>::default();
        base.set_request_matcher_builder(PathPatternRequestMatcherBuilder::default());

        Self { base }
    }
}

impl Deref for IgnoredRequestConfigurer {
    type Target = BaseRequestMatcherRegistry<Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for IgnoredRequestConfigurer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Warns when a filter chain of the proxy has no filters.
///
/// A filter chain without filters is normally the chain of an ignored request,
/// which leaves the requests it matches unsecured.
struct WebSecurityFilterChainValidator;

impl FilterChainValidator for WebSecurityFilterChainValidator {
    fn validate(&self, filter_chain_proxy: &FilterChainProxy) {
        for security_filter_chain in filter_chain_proxy.filter_chains() {
            if security_filter_chain.get_filters().is_empty() {
                warn!(
                    "A security filter chain has no filters, so the requests it matches are not \
                     secured. Map the request to allow all users with the authorization rules of \
                     the HTTP security instead."
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{IgnoredRequestConfigurer, WebSecurity};
    use crate::{
        config::security_builder::SecurityBuilder,
        web::{
            default_security_filter_chain::DefaultSecurityFilterChain,
            firewall::{
                http_status_request_rejected_handler::HttpStatusRequestRejectedHandler,
                request_rejected_handler::RequestRejectedHandler,
                strict_http_firewall::StrictHttpFirewall, HttpFirewall,
            },
            security_filter_chain::SecurityFilterChain,
            util::matcher::AnyRequestMatcher,
        },
    };

    /// A builder that creates one filter chain matching any request.
    struct AnyRequestSecurityFilterChainBuilder;

    impl SecurityBuilder<DefaultSecurityFilterChain> for AnyRequestSecurityFilterChainBuilder {
        fn build(&mut self) -> DefaultSecurityFilterChain {
            DefaultSecurityFilterChain::new(AnyRequestMatcher::instance(), Vec::new())
        }
    }

    #[test]
    fn builds_a_filter_chain_proxy_from_the_registered_builders() {
        let mut web_security = WebSecurity::default();
        web_security
            .ignoring()
            .request_matchers(vec!["/resources/**"]);
        web_security.add_security_filter_chain_builder(AnyRequestSecurityFilterChainBuilder);

        let filter_chain_proxy = web_security.build();

        // The ignored request is the first chain, and the filter chain the
        // builder contributes follows it.
        assert_eq!(filter_chain_proxy.filter_chains().len(), 2);
        assert!(filter_chain_proxy.filter_chains()[0]
            .get_filters()
            .is_empty());
        assert!(filter_chain_proxy.filter_chains()[1]
            .get_filters()
            .is_empty());
    }

    #[test]
    fn builds_a_filter_chain_proxy_from_a_filter_chain_that_is_already_built() {
        let mut web_security = WebSecurity::default();
        let security_filter_chain: Arc<dyn SecurityFilterChain> = Arc::new(
            DefaultSecurityFilterChain::new(AnyRequestMatcher::instance(), Vec::new()),
        );
        web_security.add_security_filter_chain(security_filter_chain);

        let filter_chain_proxy = web_security.build();

        assert_eq!(filter_chain_proxy.filter_chains().len(), 1);
    }

    #[test]
    fn uses_the_configured_firewall_and_request_rejected_handler() {
        let mut web_security = WebSecurity::default();
        web_security.add_security_filter_chain_builder(AnyRequestSecurityFilterChainBuilder);

        let http_firewall: Arc<dyn HttpFirewall> = Arc::new(StrictHttpFirewall::default());
        let request_rejected_handler: Arc<dyn RequestRejectedHandler> =
            Arc::new(HttpStatusRequestRejectedHandler::default());
        web_security
            .http_firewall(http_firewall.clone())
            .request_rejected_handler(request_rejected_handler.clone());

        let filter_chain_proxy = web_security.build();

        assert!(Arc::ptr_eq(&filter_chain_proxy.firewall, &http_firewall));
        assert!(Arc::ptr_eq(
            &filter_chain_proxy.request_rejected_handler,
            &request_rejected_handler
        ));
    }

    #[test]
    fn runs_the_post_build_action_after_the_proxy_is_built() {
        let built = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let mut web_security = WebSecurity::default();
        web_security.add_security_filter_chain_builder(AnyRequestSecurityFilterChainBuilder);
        let flag = Arc::clone(&built);
        web_security.post_build_action(move || {
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let _ = web_security.build();

        assert!(built.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    #[should_panic(expected = "At least one SecurityBuilder<DefaultSecurityFilterChain>")]
    fn refuses_to_build_without_a_security_filter_chain_builder() {
        let mut web_security = WebSecurity::default();

        let _ = web_security.build();
    }

    #[test]
    fn an_empty_configurer_registers_no_request_matcher() {
        let mut configurer = IgnoredRequestConfigurer::new();
        assert!(configurer.take_request_matchers().is_empty());
    }
}
