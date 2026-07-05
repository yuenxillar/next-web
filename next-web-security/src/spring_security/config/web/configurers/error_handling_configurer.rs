use std::{collections::BTreeMap, sync::Arc};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::base_http_configurer::BaseHttpConfigurer,
            http_security_builder::HttpSecurityBuilder,
        },
    },
    web::{
        access::{
            AccessDeniedHandler, AccessDeniedHandlerImpl, Builder,
            RequestMatcherDelegatingAccessDeniedHandler,
        },
        default_security_filter_chain::DefaultSecurityFilterChain,
        savedrequest::{HttpSessionRequestCache, RequestCache},
        AuthenticationEntryPoint,
    },
};

/// Configures exception/error handling — authentication entry point,
/// access denied handler. Creates `ExceptionTranslationFilter`.
///
/// Activated by default with `@EnableWebSecurity`. Use `.disable()` to opt out.
#[derive(Clone)]
pub struct ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Custom access denied handler. Falls back to `AccessDeniedHandlerImpl`.
    access_denied_handler: Option<Arc<dyn AccessDeniedHandler>>,
    /// Custom authentication entry point. Falls back to `Http403ForbiddenEntryPoint`.
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    /// Access denied page URL (creates a redirecting handler).
    access_denied_page: Option<String>,

    default_denied_handler_mappings: BTreeMap<String, Arc<dyn AccessDeniedHandler>>,
    missing_authorities_handler_builder: Option<Builder>,

    base_http_configurer: BaseHttpConfigurer<Self, H>,
}

impl<H> ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    pub fn get_access_denied_handler(&self, http: &H) -> Arc<dyn AccessDeniedHandler> {
        self.access_denied_handler
            .as_ref()
            .map(Clone::clone)
            .unwrap_or(self.create_default_denied_handler(http))
    }

    /// Set a custom `AccessDeniedHandler`.
    pub fn access_denied_handler(mut self, h: Arc<dyn AccessDeniedHandler>) -> Self {
        self.access_denied_handler = Some(h);
        self
    }

    /// Set a redirect page for access denied (e.g. `"/errors/access-denied"`).
    pub fn access_denied_page(mut self, page: &str) -> Self {
        self.access_denied_page = Some(page.to_string());
        self
    }

    /// Set a custom `AuthenticationEntryPoint`.
    pub fn authentication_entry_point(mut self, ep: Arc<dyn AuthenticationEntryPoint>) -> Self {
        self.authentication_entry_point = Some(ep);
        self
    }

    fn create_default_denied_handler(&self, http: &H) -> Arc<dyn AccessDeniedHandler> {
        let default = self.create_default_access_denied_handler(http);

        let mut denied_handler = match self.missing_authorities_handler_builder.as_ref() {
            Some(builder) => builder.build(),
            None => return default,
        };

        denied_handler.set_request_cache(self.get_request_cache(http));
        denied_handler.set_default_access_denied_handler(default);

        Arc::new(denied_handler)
    }

    fn create_default_access_denied_handler(&self, _http: &H) -> Arc<dyn AccessDeniedHandler> {
        if self.default_denied_handler_mappings.is_empty() {
            return Arc::new(AccessDeniedHandlerImpl::default());
        }

        if self.default_denied_handler_mappings.len() == 1 {
            return self
                .default_denied_handler_mappings
                .values()
                .next()
                .map(Clone::clone)
                .unwrap();
        }

        Arc::new(RequestMatcherDelegatingAccessDeniedHandler::new(
            self.default_denied_handler_mappings.clone(),
            Arc::new(AccessDeniedHandlerImpl::default()),
        ))
    }

    fn get_request_cache(&self, http: &H) -> Arc<dyn RequestCache> {
        http.shared_object::<Arc<dyn RequestCache>>()
            .map(Clone::clone)
            .unwrap_or(Arc::new(HttpSessionRequestCache::default()))
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
            access_denied_page: None,
            default_denied_handler_mappings: BTreeMap::new(),
            missing_authorities_handler_builder: None,

            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H>> for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H> {
        &self.base_http_configurer
    }
    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<ErrorHandlingConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_object()
    }
    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base_http_configurer.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for ErrorHandlingConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, _http: &mut H) {
        // ExceptionTranslationFilter is deferred until the filter is implemented.
        // When ready:
        //   let entry_point = self.authentication_entry_point.clone()
        //       .unwrap_or_else(|| Arc::new(Http403ForbiddenEntryPoint::default()));
        //   let filter = ExceptionTranslationFilter::new(entry_point);
        //   if let Some(h) = &self.access_denied_handler { filter.set_access_denied_handler(h.clone()); }
        //   http.add_filter(filter);
        //
        // All DSL fields (access_denied_handler, authentication_entry_point,
        // access_denied_page) are preserved and ready for filter construction.
    }
}
