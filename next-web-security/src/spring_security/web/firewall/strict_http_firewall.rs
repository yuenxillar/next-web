use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::web::firewall::request_rejected_error::RequestRejectedError;

use super::http_firewall::{FirewalledRequest, HttpFirewall};

#[derive(Clone)]
pub struct StrictHttpFirewall {
    allowed_methods: std::collections::HashSet<String>,
}

impl StrictHttpFirewall {
    pub fn set_unsafe_allow_any_http_method(&mut self, allow: bool) {
        self.allowed_methods = if allow {
            std::collections::HashSet::new()
        } else {
            Self::default().allowed_methods
        };
    }

    pub fn set_allowed_http_methods<I, S>(&mut self, methods: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_methods = methods.into_iter().map(Into::into).collect();
    }

    pub fn allowed_http_methods(&self) -> &std::collections::HashSet<String> {
        &self.allowed_methods
    }
}

impl HttpFirewall for StrictHttpFirewall {
    fn validate_request(&self, request: &dyn HttpRequest) -> Result<(), RequestRejectedError> {
        let method = request.method().to_string();
        if !self.allowed_methods.is_empty() && !self.allowed_methods.contains(&method) {
            return Err(RequestRejectedError(format!(
                "HTTP method {method} is not allowed"
            )));
        }
        let path = request.path();
        let lower = path.to_ascii_lowercase();
        if path.chars().any(|c| c.is_ascii_control()) || !path.is_ascii() {
            return Err(RequestRejectedError(
                "URL contains non-printable characters".into(),
            ));
        }
        if path.split('/').any(|s| s == "." || s == "..") {
            return Err(RequestRejectedError("URL is not normalized".into()));
        }
        for forbidden in [";", "%3b", "%2f", "%5c", "%00", "%25", "//"] {
            if path.contains(forbidden) || lower.contains(forbidden) {
                return Err(RequestRejectedError(format!(
                    "URL contains forbidden sequence {forbidden}"
                )));
            }
        }
        Ok(())
    }

    fn get_firewalled_request<'a>(
        &self,
        request: &'a mut dyn HttpRequest,
    ) -> Box<dyn FirewalledRequest + 'a> {
        Box::new(StrictFirewalledRequest { request })
    }

    /// Provides the response which will be passed through the filter chain.
    /// esponse the original response
    /// return either the original response or a replacement/wrapper.
    ///
    fn get_firewalled_response<'a>(
        &self,
        response: &'a mut dyn HttpResponse,
    ) -> Box<&'a mut dyn HttpResponse> {
        Box::new(response)
    }
}

/// Request wrapper returned after strict validation succeeds.
///
/// StrictHttpFirewall currently validates without rewriting request fields, so
/// reset is intentionally a no-op. Keeping the wrapper lifecycle explicit
/// matches Spring's contract and allows normalization to be added safely.
pub struct StrictFirewalledRequest<'a> {
    request: &'a mut dyn HttpRequest,
}

impl FirewalledRequest for StrictFirewalledRequest<'_> {
    fn request_mut(&mut self) -> &mut dyn HttpRequest {
        self.request
    }

    fn reset(&mut self) {}
}

impl Default for StrictHttpFirewall {
    fn default() -> Self {
        Self {
            allowed_methods: ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}
