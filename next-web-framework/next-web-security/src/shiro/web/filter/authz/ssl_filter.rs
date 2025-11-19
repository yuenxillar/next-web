use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    error::BoxError,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::object::Object,
    web::filter::{
        access_control_filter::AccessControlFilterExt, advice_filter::AdviceFilterExt,
        authz::port_filter::PortFilter, once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct SslFilter {
    hsts: HSTS,
    port_filter: PortFilter,
}

impl SslFilter {
    pub const DEFAULT_HTTPS_PORT: u16 = 443;
    pub const HTTPS_SCHEME: &str = "https";

    pub fn get_scheme<'a>(&self, _request_scheme: Option<&'a str>, port: u16) -> &'a str {
        if port == PortFilter::DEFAULT_HTTP_PORT {
            return PortFilter::HTTP_SCHEME;
        } else {
            return Self::HTTPS_SCHEME;
        }
    }
}

#[async_trait]
impl AccessControlFilterExt for SslFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        self.port_filter
            .is_access_allowed(request, response, mapped_value)
            .await
            && request.is_secure()
    }
}

#[async_trait]
impl AdviceFilterExt for SslFilter {
    async fn post_handle(
        &self,
        _request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        if self.hsts.is_enabled() {
            let mut directives = String::with_capacity(64);
            directives.push_str("max-age=");
            directives.push_str(&self.hsts.get_max_age().to_string());

            if self.hsts.is_include_sub_domains() {
                directives.push_str("; includeSubDomains");
            }

            response.append_header(HSTS::HTTP_HEADER.as_bytes(), &directives);
        }
        Ok(())
    }
}

impl Required<OncePerRequestFilter> for SslFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .port_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .port_filter
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for SslFilter {
    fn name(&self) -> &str {
        "SslFilter"
    }
}

impl Deref for SslFilter {
    type Target = PortFilter;

    fn deref(&self) -> &Self::Target {
        &self.port_filter
    }
}

impl DerefMut for SslFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.port_filter
    }
}

impl Default for SslFilter {
    fn default() -> Self {
        let mut port_filter: PortFilter = Default::default();
        port_filter.set_port(Self::DEFAULT_HTTPS_PORT);
        Self {
            port_filter,
            hsts: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct HSTS {
    enabled: bool,
    max_age: u32,
    include_sub_domains: bool,
}

impl HSTS {
    pub const HTTP_HEADER: &str = "Strict-Transport-Security";
    pub const DEFAULT_ENABLED: bool = false;
    pub const DEFAULT_MAX_AGE: u32 = 31536000;
    pub const DEFAULT_INCLUDE_SUB_DOMAINS: bool = false;

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn get_max_age(&self) -> u32 {
        self.max_age
    }

    pub fn set_max_age(&mut self, max_age: u32) {
        self.max_age = max_age;
    }

    pub fn is_include_sub_domains(&self) -> bool {
        self.include_sub_domains
    }

    pub fn set_include_sub_domains(&mut self, include_sub_domains: bool) {
        self.include_sub_domains = include_sub_domains;
    }
}

impl Default for HSTS {
    fn default() -> Self {
        Self {
            enabled: Self::DEFAULT_ENABLED,
            max_age: Self::DEFAULT_MAX_AGE,
            include_sub_domains: Self::DEFAULT_INCLUDE_SUB_DOMAINS,
        }
    }
}
