use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    web::{
        access::{
            AccessDeniedHandler, AccessDeniedHandlerImpl,
            DelegatingMissingAuthorityAccessDeniedHandler,
            DelegatingMissingAuthorityAccessDeniedHandlerBuilder, ErrorTranslationFilter,
            RequestMatcherDelegatingAccessDeniedHandler,
        },
        authentication::{
            DelegatingAuthenticationEntryPoint, DelegatingAuthenticationEntryPointBuilder,
            Http403ForbiddenEntryPoint,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::{HttpSessionRequestCache, RequestCache},
        util::matcher::RequestMatcher,
        AuthenticationEntryPoint,
    },
};

/// Adds exception handling for Spring Security related exceptions to an application. All
/// properties have reasonable defaults, so no additional configuration is required other
/// than applying this `SecurityConfigurer`.
///
/// # Security Filters
///
/// The following Filters are populated:
///
/// * `ExceptionTranslationFilter`
///
/// # Shared Objects Created
///
/// No shared objects are created.
///
/// # Shared Objects Used
///
/// The following shared objects are used:
///
/// * If no explicit `RequestCache` is provided, a `RequestCache` shared object is used
///   to replay the request after authentication is successful.
/// * `AuthenticationEntryPoint` - see `authentication_entry_point`.
#[derive(Clone)]
pub struct ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    access_denied_handler: Option<Arc<dyn AccessDeniedHandler>>,
    default_entry_point: Option<DelegatingAuthenticationEntryPointBuilder>,

    default_denied_handler_mappings: Vec<(Arc<dyn RequestMatcher>, Arc<dyn AccessDeniedHandler>)>,
    missing_authorities_handler_builder:
        Option<DelegatingMissingAuthorityAccessDeniedHandlerBuilder>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Shortcut to specify the `AccessDeniedHandler` to be used as a specific error
    /// page.
    ///
    /// # Arguments
    ///
    /// * `access_denied_url` - The URL to the access denied page (i.e. /errors/401).
    pub fn access_denied_page(&mut self, access_denied_url: impl Into<String>) -> &mut Self {
        let mut access_denied_handler = AccessDeniedHandlerImpl::default();
        access_denied_handler.set_error_page(access_denied_url);
        self.access_denied_handler(Arc::new(access_denied_handler))
    }

    /// Specifies the `AccessDeniedHandler` to be used.
    ///
    /// # Arguments
    ///
    /// * `access_denied_handler` - The `AccessDeniedHandler` to be used.
    pub fn access_denied_handler(
        &mut self,
        access_denied_handler: Arc<dyn AccessDeniedHandler>,
    ) -> &mut Self {
        self.access_denied_handler = Some(access_denied_handler);
        self
    }

    /// Sets a default `AccessDeniedHandler` to be used which prefers being invoked for
    /// the provided `RequestMatcher`. If only a single default `AccessDeniedHandler` is
    /// specified, it will be what is used for the default `AccessDeniedHandler`. If
    /// multiple default `AccessDeniedHandler` instances are configured, then a
    /// `RequestMatcherDelegatingAccessDeniedHandler` will be used.
    ///
    /// # Arguments
    ///
    /// * `denied_handler` - The `AccessDeniedHandler` to use.
    /// * `preferred_matcher` - The `RequestMatcher` for this default
    ///   `AccessDeniedHandler`.
    pub fn default_access_denied_handler_for(
        &mut self,
        denied_handler: Arc<dyn AccessDeniedHandler>,
        preferred_matcher: Arc<dyn RequestMatcher>,
    ) -> &mut Self {
        self.default_denied_handler_mappings
            .push((preferred_matcher, denied_handler));
        self
    }

    /// Sets a default `AuthenticationEntryPoint` to be used which prefers being invoked
    /// for the provided missing `GrantedAuthority`.
    ///
    /// # Arguments
    ///
    /// * `entry_point` - The `AuthenticationEntryPoint` to use for the given authority.
    /// * `authority` - The authority string.
    pub fn default_denied_handler_for_missing_authority(
        &mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
        authority: impl Into<String>,
    ) -> &mut Self {
        if self.missing_authorities_handler_builder.is_none() {
            self.missing_authorities_handler_builder =
                Some(DelegatingMissingAuthorityAccessDeniedHandler::builder());
        }
        if let Some(builder) = self.missing_authorities_handler_builder.as_mut() {
            builder.add_entry_point_for(entry_point, authority);
        }
        self
    }

    /// Sets a default `AuthenticationEntryPoint` to be used which prefers being invoked
    /// for the provided missing `GrantedAuthority`.
    ///
    /// # Arguments
    ///
    /// * `entry_point_builder_fn` - A function that configures a
    ///   `DelegatingAuthenticationEntryPointBuilder` to use for the given authority.
    /// * `authority` - The authority string.
    pub fn default_denied_handler_for_missing_authority_with_builder<F>(
        &mut self,
        entry_point_builder_fn: F,
        authority: &str,
    ) -> &mut Self
    where
        F: FnOnce(&mut DelegatingAuthenticationEntryPointBuilder),
    {
        if self.missing_authorities_handler_builder.is_none() {
            self.missing_authorities_handler_builder =
                Some(DelegatingMissingAuthorityAccessDeniedHandler::builder());
        }
        if let Some(builder) = self.missing_authorities_handler_builder.as_mut() {
            builder.add_entry_point_for_with_builder(entry_point_builder_fn, authority);
        }
        self
    }

    /// Sets the `AuthenticationEntryPoint` to be used.
    ///
    /// If no `authentication_entry_point` is specified, then
    /// `default_authentication_entry_point_for` will be used. The first
    /// `AuthenticationEntryPoint` will be used as the default if no matches were found.
    ///
    /// If that is not provided, defaults to `Http403ForbiddenEntryPoint`.
    ///
    /// # Arguments
    ///
    /// * `authentication_entry_point` - The `AuthenticationEntryPoint` to use.
    pub fn authentication_entry_point(
        &mut self,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> &mut Self {
        self.authentication_entry_point = Some(authentication_entry_point);
        self
    }

    /// Sets a default `AuthenticationEntryPoint` to be used which prefers being invoked
    /// for the provided `RequestMatcher`. If only a single default
    /// `AuthenticationEntryPoint` is specified, it will be what is used for the default
    /// `AuthenticationEntryPoint`. If multiple default `AuthenticationEntryPoint`
    /// instances are configured, then a `DelegatingAuthenticationEntryPoint` will be
    /// used.
    ///
    /// # Arguments
    ///
    /// * `entry_point` - The `AuthenticationEntryPoint` to use.
    /// * `preferred_matcher` - The `RequestMatcher` for this default
    ///   `AuthenticationEntryPoint`.
    pub fn default_authentication_entry_point_for(
        &mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
        preferred_matcher: Arc<dyn RequestMatcher>,
    ) -> &mut Self {
        if self.default_entry_point.is_none() {
            self.default_entry_point = Some(DelegatingAuthenticationEntryPoint::builder());
        }
        if let Some(builder) = self.default_entry_point.as_mut() {
            builder.add_entry_point_for(entry_point, preferred_matcher);
        }
        self
    }

    /// Gets any explicitly configured `AuthenticationEntryPoint`.
    pub fn get_authentication_entry_point(&self) -> Option<&Arc<dyn AuthenticationEntryPoint>> {
        self.authentication_entry_point.as_ref()
    }

    /// Gets the `AccessDeniedHandler` that is configured.
    pub fn get_access_denied_handler(&self) -> Option<&Arc<dyn AccessDeniedHandler>> {
        self.access_denied_handler.as_ref()
    }

    /// Gets the `AccessDeniedHandler` according to the rules specified by
    /// `access_denied_handler`.
    fn get_access_denied_handler_internal(&mut self, http: &mut H) -> Arc<dyn AccessDeniedHandler> {
        if let Some(denied_handler) = self.access_denied_handler.take() {
            return denied_handler;
        }
        self.create_default_denied_handler(http)
    }

    /// Gets the `AuthenticationEntryPoint` according to the rules specified by
    /// `authentication_entry_point`.
    fn get_authentication_entry_point_internal(
        &mut self,
        http: &mut H,
    ) -> Arc<dyn AuthenticationEntryPoint> {
        if let Some(entry_point) = self.authentication_entry_point.take() {
            return entry_point.clone();
        }
        self.create_default_entry_point(http)
    }

    /// Creates the default `AccessDeniedHandler`.
    fn create_default_denied_handler(&mut self, http: &mut H) -> Arc<dyn AccessDeniedHandler> {
        let defaults = self.create_default_access_denied_handler(http);
        match self.missing_authorities_handler_builder.as_mut() {
            None => defaults,
            Some(builder) => {
                let mut denied_handler = builder.build();
                denied_handler.set_request_cache(self.get_request_cache(http));
                denied_handler.set_default_access_denied_handler(defaults);
                Arc::new(denied_handler)
            }
        }
    }

    /// Creates the default `AccessDeniedHandler` from the configured mappings.
    fn create_default_access_denied_handler(
        &mut self,
        _http: &mut H,
    ) -> Arc<dyn AccessDeniedHandler> {
        if self.default_denied_handler_mappings.is_empty() {
            return Arc::new(AccessDeniedHandlerImpl::default());
        }
        if self.default_denied_handler_mappings.len() == 1 {
            return self.default_denied_handler_mappings[0].1.clone();
        }
        Arc::new(RequestMatcherDelegatingAccessDeniedHandler::new(
            std::mem::take(&mut self.default_denied_handler_mappings),
            Arc::new(AccessDeniedHandlerImpl::default()),
        ))
    }

    /// Creates the default `AuthenticationEntryPoint`.
    fn create_default_entry_point(&mut self, _http: &mut H) -> Arc<dyn AuthenticationEntryPoint> {
        match self.default_entry_point.as_mut() {
            None => Arc::new(Http403ForbiddenEntryPoint::default()),
            Some(builder) => builder.build(),
        }
    }

    /// Gets the `RequestCache` to use. If one is defined using `request_cache`, then it
    /// is used. Otherwise, an attempt to find a `RequestCache` shared object is made. If
    /// that fails, an `HttpSessionRequestCache` is used.
    fn get_request_cache(&self, http: &H) -> Arc<dyn RequestCache> {
        http.shared_object::<Arc<dyn RequestCache>>()
            .map(Clone::clone)
            .unwrap_or(Arc::new(HttpSessionRequestCache::default()))
    }
}

impl<H> Deref for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }
    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let entry_point = self.get_authentication_entry_point_internal(http);
        let mut error_translation_filter =
            ErrorTranslationFilter::new(entry_point, self.get_request_cache(http));

        let denied_handler = self.get_access_denied_handler_internal(http);
        error_translation_filter.set_access_denied_handler(denied_handler);
        error_translation_filter.set_security_context_holder_strategy(
            self.base.get_security_context_holder_strategy().to_owned(),
        );

        // let error_translation_filter = self
        //     .inner
        //     .post_process(error_translation_filter);
        http.add_filter(error_translation_filter);
    }
}

impl<H> Default for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            access_denied_handler: None,
            authentication_entry_point: None,
            default_entry_point: None,
            default_denied_handler_mappings: Vec::new(),
            missing_authorities_handler_builder: None,

            base: Default::default(),
        }
    }
}
