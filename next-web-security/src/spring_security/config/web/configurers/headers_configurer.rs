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
        default_security_filter_chain::DefaultSecurityFilterChain,
        header::{
            writers::{
                CacheControlHeadersWriter, ContentSecurityPolicyHeaderWriter,
                CrossOriginEmbedderPolicy, CrossOriginEmbedderPolicyHeaderWriter,
                CrossOriginOpenerPolicy, CrossOriginOpenerPolicyHeaderWriter,
                CrossOriginResourcePolicy, CrossOriginResourcePolicyHeaderWriter,
                FeaturePolicyHeaderWriter, HstsHeaderWriter, PermissionsPolicyHeaderWriter,
                ReferrerPolicy, ReferrerPolicyHeaderWriter, XContentTypeOptionsHeaderWriter,
                XFrameOptionsHeaderWriter, XFrameOptionsMode, XXssHeaderValue,
                XXssProtectionHeaderWriter,
            },
            HeaderWriter, HeaderWriterFilter,
        },
        util::matcher::RequestMatcher,
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
{
    header_writers: Vec<Arc<dyn HeaderWriter>>,
    content_type_options: ContentTypeOptionsConfig,
    xss_protection: XXssConfig,
    cache_control: CacheControlConfig,
    hsts: HstsConfig,
    frame_options: FrameOptionsConfig,
    content_security_policy: ContentSecurityPolicyConfig,
    referrer_policy: ReferrerPolicyConfig,
    feature_policy: FeaturePolicyConfig,
    permissions_policy: PermissionsPolicyConfig,
    cross_origin_opener_policy: CrossOriginOpenerPolicyConfig,
    cross_origin_embedder_policy: CrossOriginEmbedderPolicyConfig,
    cross_origin_resource_policy: CrossOriginResourcePolicyConfig,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Disable all default headers. Call this first, then selectively enable.
    pub fn defaults_disabled(mut self) -> Self {
        self.content_type_options.disable();
        self.xss_protection.disable();
        self.cache_control.disable();
        self.hsts.disable();
        self.frame_options.disable();

        self
    }

    /// Add a custom `HeaderWriter`.
    pub fn add_header_writer(&mut self, writer: Arc<dyn HeaderWriter>) -> &mut Self {
        self.header_writers.push(writer);

        self
    }

    pub fn content_type_options<F>(&mut self, mut content_type_options: F) -> &mut Self
    where
        F: FnMut(&mut ContentTypeOptionsConfig),
    {
        content_type_options(&mut self.content_type_options);

        self
    }

    pub fn xss_protection<F>(&mut self, mut xss_protection: F) -> &mut Self
    where
        F: FnMut(&mut XXssConfig),
    {
        xss_protection(&mut self.xss_protection);

        self
    }

    pub fn cache_control<F>(&mut self, mut cache_control: F) -> &mut Self
    where
        F: FnMut(&mut CacheControlConfig),
    {
        cache_control(&mut self.cache_control);

        self
    }

    pub fn http_strict_transport_security<F>(
        mut self,
        mut http_strict_transport_security: F,
    ) -> Self
    where
        F: FnMut(&mut HstsConfig),
    {
        http_strict_transport_security(&mut self.hsts);

        self
    }

    pub fn frame_options<F>(mut self, mut frame_options: F) -> Self
    where
        F: FnMut(&mut FrameOptionsConfig),
    {
        frame_options(&mut self.frame_options);

        self
    }

    pub fn content_security_policy<F>(mut self, mut content_security_policy: F) -> Self
    where
        F: FnMut(&mut ContentSecurityPolicyConfig),
    {
        self.content_security_policy.writer = Some(Default::default());
        content_security_policy(&mut self.content_security_policy);

        self
    }

    pub fn referrer_policy<F>(mut self, mut referrer_policy: F) -> Self
    where
        F: FnMut(&mut ReferrerPolicyConfig),
    {
        self.referrer_policy.writer = Some(Default::default());
        referrer_policy(&mut self.referrer_policy);

        self
    }

    pub fn permissions_policy_header<F>(mut self, mut permissions_policy_header: F) -> Self
    where
        F: FnMut(&mut PermissionsPolicyConfig),
    {
        self.permissions_policy.writer = Some(Default::default());
        permissions_policy_header(&mut self.permissions_policy);

        self
    }

    pub fn cross_origin_opener_policy<F>(mut self, mut cross_origin_opener_policy: F) -> Self
    where
        F: FnMut(&mut CrossOriginOpenerPolicyConfig),
    {
        self.cross_origin_opener_policy.writer = Some(Default::default());
        cross_origin_opener_policy(&mut self.cross_origin_opener_policy);

        self
    }

    pub fn cross_origin_embedder_policy<F>(mut self, mut cross_origin_embedder_policy: F) -> Self
    where
        F: FnMut(&mut CrossOriginEmbedderPolicyConfig),
    {
        self.cross_origin_embedder_policy.writer = Some(Default::default());
        cross_origin_embedder_policy(&mut self.cross_origin_embedder_policy);

        self
    }

    pub fn cross_origin_resource_policy<F>(mut self, mut cross_origin_resource_policy: F) -> Self
    where
        F: FnMut(&mut CrossOriginResourcePolicyConfig),
    {
        self.cross_origin_resource_policy.writer = Some(Default::default());
        cross_origin_resource_policy(&mut self.cross_origin_resource_policy);

        self
    }

    fn create_header_writer_filter(&mut self) -> HeaderWriterFilter {
        let writers = self.get_header_writers();
        assert!(!writers.is_empty(), "Headers security is enabled, but no headers will be added. Either add headers or disable headers security");

        // todo!(postProcess);
        HeaderWriterFilter::new(writers)
    }

    /// Build the list of configured header writers.
    fn get_header_writers(&mut self) -> Vec<Arc<dyn HeaderWriter>> {
        let mut writers: Vec<Arc<dyn HeaderWriter>> = Vec::new();

        Self::add_if_not_null(
            &mut writers,
            self.content_type_options
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.xss_protection
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.cache_control
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.hsts
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.frame_options
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.content_security_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.referrer_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.feature_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.permissions_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.cross_origin_opener_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.cross_origin_embedder_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        Self::add_if_not_null(
            &mut writers,
            self.cross_origin_resource_policy
                .writer
                .take()
                .map(|s| Arc::new(s) as Arc<dyn HeaderWriter>),
        );
        writers.extend(std::mem::take(&mut self.header_writers));

        writers
    }

    fn add_if_not_null<T>(values: &mut Vec<T>, value: Option<T>) {
        if let Some(value) = value {
            values.push(value);
        }
    }
}

impl<H> Default for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            header_writers: Default::default(),
            content_type_options: Default::default(),
            xss_protection: Default::default(),
            cache_control: Default::default(),
            hsts: Default::default(),
            frame_options: Default::default(),
            content_security_policy: Default::default(),
            referrer_policy: Default::default(),
            feature_policy: Default::default(),
            permissions_policy: Default::default(),
            cross_origin_opener_policy: Default::default(),
            cross_origin_embedder_policy: Default::default(),
            cross_origin_resource_policy: Default::default(),

            base: Default::default(),
        }
    }
}

impl<H> Deref for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for HeadersConfigurer<H>
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

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for HeadersConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, _http: &mut H) {}

    fn configure(&mut self, http: &mut H) {
        let filter = self.create_header_writer_filter();
        http.add_filter(filter);
    }
}

#[derive(Clone)]
pub struct ContentTypeOptionsConfig {
    writer: Option<XContentTypeOptionsHeaderWriter>,
}

impl ContentTypeOptionsConfig {
    /// Removes the X-Content-Type-Options header.
    pub fn disable(&mut self) {
        self.writer = None;
    }

    fn enable(&mut self) {
        if self.writer.is_none() {
            self.writer = Some(Default::default());
        }
    }
}

impl Default for ContentTypeOptionsConfig {
    fn default() -> Self {
        let mut config = Self {
            writer: Default::default(),
        };
        config.enable();

        config
    }
}

#[derive(Clone)]
pub struct XXssConfig {
    writer: Option<XXssProtectionHeaderWriter>,
}

impl XXssConfig {
    /// Sets the value of the X-XSS-PROTECTION header.
    pub fn header_value(mut self, header_value: XXssHeaderValue) -> Self {
        if let Some(ref mut writer) = self.writer {
            writer.set_header_value(header_value);
        }
        self
    }

    /// Disables X-XSS-Protection header.
    pub fn disable(&mut self) {
        self.writer = None;
    }

    fn enable(&mut self) {
        if self.writer.is_none() {
            self.writer = Some(Default::default());
        }
    }
}

impl Default for XXssConfig {
    fn default() -> Self {
        let mut config = Self {
            writer: Default::default(),
        };
        config.enable();

        config
    }
}

#[derive(Clone)]
pub struct CacheControlConfig {
    writer: Option<CacheControlHeadersWriter>,
}

impl CacheControlConfig {
    /// Disables Cache Control.
    pub fn disable(&mut self) {
        self.writer = None;
    }

    fn enable(&mut self) {
        if self.writer.is_none() {
            self.writer = Some(Default::default());
        }
    }
}

impl Default for CacheControlConfig {
    fn default() -> Self {
        let mut config = Self {
            writer: Default::default(),
        };
        config.enable();

        config
    }
}

#[derive(Clone)]
pub struct HstsConfig {
    writer: Option<HstsHeaderWriter>,
}

impl HstsConfig {
    /// Sets the value (in seconds) for the max-age directive.
    pub fn max_age_in_seconds(mut self, max_age_in_seconds: u64) -> Self {
        if let Some(ref mut writer) = self.writer {
            writer.set_max_age_in_seconds(max_age_in_seconds);
        }
        self
    }

    /// Sets the RequestMatcher used to determine if the "Strict-Transport-Security" should be added.
    pub fn request_matcher(mut self, request_matcher: Arc<dyn RequestMatcher>) -> Self {
        if let Some(ref mut writer) = self.writer {
            writer.set_request_matcher(request_matcher);
        }
        self
    }

    /// If true, subdomains should be considered HSTS Hosts too.
    pub fn include_sub_domains(mut self, include_sub_domains: bool) -> Self {
        if let Some(ref mut writer) = self.writer {
            writer.set_include_sub_domains(include_sub_domains);
        }
        self
    }

    /// If true, preload will be included in HSTS Header.
    pub fn preload(mut self, preload: bool) -> Self {
        if let Some(ref mut writer) = self.writer {
            writer.set_preload(preload);
        }
        self
    }

    /// Disables Strict Transport Security.
    pub fn disable(&mut self) {
        self.writer = None;
    }

    fn enable(&mut self) {
        if self.writer.is_none() {
            self.writer = Some(Default::default());
        }
    }
}

impl Default for HstsConfig {
    fn default() -> Self {
        let mut config = Self {
            writer: Default::default(),
        };
        config.enable();

        config
    }
}

#[derive(Clone)]
pub struct FrameOptionsConfig {
    writer: Option<XFrameOptionsHeaderWriter>,
}

impl FrameOptionsConfig {
    /// Specify to DENY framing any content from this application.
    pub fn deny(&mut self) {
        self.writer = Some(XFrameOptionsHeaderWriter::new(XFrameOptionsMode::Deny));
    }

    /// Specify to allow any request that comes from the same origin to frame this application.
    pub fn same_origin(&mut self) {
        self.writer = Some(XFrameOptionsHeaderWriter::new(
            XFrameOptionsMode::SameoriGin,
        ));
    }

    /// Prevents the header from being added to the response.
    pub fn disable(&mut self) {
        self.writer = None;
    }

    fn enable(&mut self) {
        if self.writer.is_none() {
            self.writer = Some(XFrameOptionsHeaderWriter::new(XFrameOptionsMode::Deny));
        }
    }
}

impl Default for FrameOptionsConfig {
    fn default() -> Self {
        let mut config = Self {
            writer: Default::default(),
        };
        config.enable();

        config
    }
}

#[derive(Clone, Default)]
pub struct ContentSecurityPolicyConfig {
    writer: Option<ContentSecurityPolicyHeaderWriter>,
}

impl ContentSecurityPolicyConfig {
    /// Sets the security policy directive(s) to be used in the response header.
    pub fn policy_directives(mut self, policy_directives: impl Into<String>) -> Self {
        self.writer
            .as_mut()
            .map(|w| w.set_policy_directives(policy_directives));
        self
    }

    /// Enables the Content-Security-Policy-Report-Only header.
    pub fn report_only(mut self) -> Self {
        self.writer.as_mut().map(|w| w.set_report_only(true));

        self
    }
}

#[derive(Clone, Default)]
pub struct ReferrerPolicyConfig {
    writer: Option<ReferrerPolicyHeaderWriter>,
}

impl ReferrerPolicyConfig {
    /// Sets the policy to be used in the response header.
    pub fn policy(mut self, policy: ReferrerPolicy) -> Self {
        self.writer.as_mut().map(|w| w.set_policy(policy));

        self
    }
}

#[derive(Clone, Default)]
pub struct FeaturePolicyConfig {
    writer: Option<FeaturePolicyHeaderWriter>,
}

impl FeaturePolicyConfig {
    #[allow(dead_code)]
    pub fn and(self) -> Self {
        self
    }
}

#[derive(Clone, Default)]
pub struct PermissionsPolicyConfig {
    writer: Option<PermissionsPolicyHeaderWriter>,
}

impl PermissionsPolicyConfig {
    /// Sets the policy to be used in the response header.
    pub fn policy(mut self, policy: impl Into<String>) -> Self {
        if let Some(writer) = self.writer.as_mut() {
            writer.set_policy(policy);
        }
        self
    }
}

#[derive(Clone, Default)]
pub struct CrossOriginOpenerPolicyConfig {
    writer: Option<CrossOriginOpenerPolicyHeaderWriter>,
}

impl CrossOriginOpenerPolicyConfig {
    /// Sets the policy to be used in the Cross-Origin-Opener-Policy header.
    pub fn policy(mut self, policy: CrossOriginOpenerPolicy) -> Self {
        self.writer.as_mut().map(|w| w.set_policy(policy));

        self
    }
}

#[derive(Clone, Default)]
pub struct CrossOriginEmbedderPolicyConfig {
    writer: Option<CrossOriginEmbedderPolicyHeaderWriter>,
}

impl CrossOriginEmbedderPolicyConfig {
    /// Sets the policy to be used in the Cross-Origin-Embedder-Policy header.
    pub fn policy(mut self, policy: CrossOriginEmbedderPolicy) -> Self {
        self.writer.as_mut().map(|w| w.set_policy(policy));

        self
    }
}

#[derive(Clone, Default)]
pub struct CrossOriginResourcePolicyConfig {
    writer: Option<CrossOriginResourcePolicyHeaderWriter>,
}

impl CrossOriginResourcePolicyConfig {
    /// Sets the policy to be used in the Cross-Origin-Resource-Policy header.
    pub fn policy(mut self, policy: CrossOriginResourcePolicy) -> Self {
        self.writer.as_mut().map(|w| w.set_policy(policy));

        self
    }
}
