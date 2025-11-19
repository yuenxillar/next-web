use std::ops::{Deref, DerefMut};

use next_web_core::{
    async_trait,
    traits::{
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
        required::Required,
    },
};

use crate::{
    core::util::{object::Object, web::WebUtils},
    web::filter::{
        access_control_filter::AccessControlFilterExt,
        advice_filter::AdviceFilterExt,
        authz::{authorization_filter::AuthorizationFilter, ssl_filter::SslFilter},
        once_per_request_filter::OncePerRequestFilter,
    },
};

#[derive(Clone)]
pub struct PortFilter {
    port: u16,
    authorization_filter: AuthorizationFilter,
}

impl PortFilter {
    pub const DEFAULT_HTTP_PORT: u16 = 80;
    pub const HTTP_SCHEME: &str = "http";

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = port;
    }

    fn to_port(&self, mapped_value: Option<&Object>) -> u16 {
        match mapped_value {
            Some(value) => {
                if let Object::Int(port) = value {
                    return *port as u16;
                }
            }
            None => {}
        };
        self.get_port()
    }

    pub fn get_scheme<'a>(&self, request_scheme: Option<&'a str>, port: u16) -> &'a str {
        if port == Self::DEFAULT_HTTP_PORT {
            return Self::HTTP_SCHEME;
        } else if port == SslFilter::DEFAULT_HTTPS_PORT {
            return SslFilter::HTTPS_SCHEME;
        } else {
            return request_scheme.unwrap_or(Self::HTTP_SCHEME);
        }
    }
}

#[async_trait]
impl AccessControlFilterExt for PortFilter {
    async fn is_access_allowed(
        &self,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let port = self.to_port(mapped_value.as_ref());
        let request_port = request.server_port();

        port == request_port.unwrap_or(80)
    }

    async fn on_access_denied(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        mapped_value: Option<Object>,
    ) -> bool {
        let port = self.to_port(mapped_value.as_ref());

        let scheme = self.get_scheme(request.scheme(), port);

        let mut url = String::from(scheme);
        url.push_str("://");
        url.push_str(&request.server_name().unwrap_or("localhost".to_string()));

        if port != Self::DEFAULT_HTTP_PORT && port != SslFilter::DEFAULT_HTTPS_PORT {
            url.push(':');
            url.push_str(&port.to_string());
        }

        url.push_str(request.path());
        if let Some(query) = request.query() {
            url.push('?');
            url.push_str(query);
        }

        WebUtils::issue_redirect(request, response, &url);

        false
    }
}

#[async_trait]
impl AdviceFilterExt for PortFilter {}

impl Required<OncePerRequestFilter> for PortFilter {
    fn get_object(&self) -> &OncePerRequestFilter {
        &self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }

    fn get_mut_object(&mut self) -> &mut OncePerRequestFilter {
        &mut self
            .access_control_filter
            .path_matching_filter
            .advice_filter
            .once_per_request_filter
    }
}

impl Named for PortFilter {
    fn name(&self) -> &str {
        "PortFilter"
    }
}

impl Deref for PortFilter {
    type Target = AuthorizationFilter;

    fn deref(&self) -> &Self::Target {
        &self.authorization_filter
    }
}

impl DerefMut for PortFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.authorization_filter
    }
}

impl Default for PortFilter {
    fn default() -> Self {
        Self {
            port: Self::DEFAULT_HTTP_PORT,
            authorization_filter: Default::default(),
        }
    }
}
