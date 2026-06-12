use std::sync::Arc;

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
        default_security_filter_chain::DefaultSecurityFilterChain,
        header::{
            writers::{
                CacheControlHeadersWriter, HstsHeaderWriter, XContentTypeOptionsHeaderWriter,
                XFrameOptionsHeaderWriter, XFrameOptionsMode, XXssProtectionHeaderWriter,
            },
            HeaderWriter, HeaderWriterFilter,
        },
    },
};

/// Configures HTTP security headers. Activated by default with `@EnableWebSecurity`.
///
/// Default headers set:
/// - `Cache-Control: no-cache, no-store, max-age=0, must-revalidate`
/// - `Pragma: no-cache`
/// - `Expires: 0`
/// - `X-Content-Type-Options: nosniff`
/// - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
/// - `X-Frame-Options: DENY`
/// - `X-XSS-Protection: 0`
///
/// Use `headers.defaults_disabled()` to remove defaults, then selectively enable headers.
#[derive(Clone)]
pub struct HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<Self, H>>,
{
    defaults_disabled: bool,
    cache_control_enabled: bool,
    content_type_options_enabled: bool,
    xss_protection_enabled: bool,
    hsts_enabled: bool,
    frame_options_enabled: bool,
    frame_options_mode: XFrameOptionsMode,

    hsts_max_age: u64,
    hsts_include_subdomains: bool,
    hsts_preload: bool,

    custom_header_writers: Vec<Arc<dyn HeaderWriter>>,

    base_http_configurer: BaseHttpConfigurer<HeadersConfigurer<H>, H>,
}

impl<H> HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HeadersConfigurer<H>, H>>,
{
    /// Disable all default headers. Call this first, then selectively enable.
    pub fn defaults_disabled(mut self) -> Self {
        self.defaults_disabled = true;
        self.cache_control_enabled = false;
        self.content_type_options_enabled = false;
        self.xss_protection_enabled = false;
        self.hsts_enabled = false;
        self.frame_options_enabled = false;
        self
    }

    /// Enable/disable `Cache-Control`, `Pragma`, and `Expires` headers.
    pub fn cache_control(mut self, enabled: bool) -> Self {
        self.cache_control_enabled = enabled;
        self
    }

    /// Enable/disable `X-Content-Type-Options: nosniff`.
    pub fn content_type_options(mut self, enabled: bool) -> Self {
        self.content_type_options_enabled = enabled;
        self
    }

    /// Enable/disable `X-XSS-Protection`.
    pub fn xss_protection(mut self, enabled: bool) -> Self {
        self.xss_protection_enabled = enabled;
        self
    }

    /// Configure HTTP Strict Transport Security (HSTS).
    pub fn http_strict_transport_security(
        mut self,
        enabled: bool,
        max_age: u64,
        include_subdomains: bool,
    ) -> Self {
        self.hsts_enabled = enabled;
        self.hsts_max_age = max_age;
        self.hsts_include_subdomains = include_subdomains;
        self
    }

    /// Enable HSTS preload.
    pub fn hsts_preload(mut self, preload: bool) -> Self {
        self.hsts_preload = preload;
        self
    }

    /// Configure `X-Frame-Options`. Use `deny()`, `same_origin()`, or set mode directly.
    pub fn frame_options(mut self, mode: XFrameOptionsMode) -> Self {
        self.frame_options_enabled = true;
        self.frame_options_mode = mode;
        self
    }

    /// Add a custom `HeaderWriter`.
    pub fn add_header_writer(mut self, writer: Arc<dyn HeaderWriter>) -> Self {
        self.custom_header_writers.push(writer);
        self
    }

    /// Build the list of configured header writers.
    fn get_header_writers(&self) -> Vec<Arc<dyn HeaderWriter>> {
        let mut writers: Vec<Arc<dyn HeaderWriter>> = Vec::new();

        if self.cache_control_enabled {
            writers.push(Arc::new(CacheControlHeadersWriter::default()));
        }
        if self.content_type_options_enabled {
            writers.push(Arc::new(XContentTypeOptionsHeaderWriter::default()));
        }
        if self.xss_protection_enabled {
            writers.push(Arc::new(XXssProtectionHeaderWriter::default()));
        }
        if self.hsts_enabled {
            let mut hsts = HstsHeaderWriter::default();
            hsts.set_max_age_in_seconds(self.hsts_max_age);
            hsts.set_include_sub_domains(self.hsts_include_subdomains);
            hsts.set_preload(self.hsts_preload);
            writers.push(Arc::new(hsts));
        }
        if self.frame_options_enabled {
            writers.push(Arc::new(XFrameOptionsHeaderWriter::new(
                self.frame_options_mode.clone(),
            )));
        }

        // Add any custom header writers
        writers.extend(self.custom_header_writers.clone());

        writers
    }
}

impl<H> Default for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    Self: Required<BaseHttpConfigurer<HeadersConfigurer<H>, H>>,
{
    fn default() -> Self {
        Self {
            defaults_disabled: false,
            // All defaults enabled (matches Spring Security behavior)
            cache_control_enabled: true,
            content_type_options_enabled: true,
            xss_protection_enabled: true,
            hsts_enabled: true,
            frame_options_enabled: true,
            frame_options_mode: XFrameOptionsMode::DenY,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            custom_header_writers: Vec::new(),
            base_http_configurer: Default::default(),
        }
    }
}

impl<H> Required<BaseHttpConfigurer<HeadersConfigurer<H>, H>> for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &BaseHttpConfigurer<HeadersConfigurer<H>, H> {
        &self.base_http_configurer
    }

    fn get_mut_object(&mut self) -> &mut BaseHttpConfigurer<HeadersConfigurer<H>, H> {
        &mut self.base_http_configurer
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for HeadersConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {
        // No init needed — header writers are self-contained.
    }

    fn configure(&mut self, http: &mut H) {
        let writers = self.get_header_writers();
        assert!(
            !writers.is_empty(),
            "At least one HeaderWriter must be configured, or call defaultsDisabled() \
             before enabling specific headers."
        );
        let filter = HeaderWriterFilter::new(writers);
        http.add_filter(filter);
    }
}
