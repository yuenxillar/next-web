use std::collections::HashSet;

use crate::http::HttpMethod;

/// Configuration for CORS processing
#[derive(Debug, Clone)]
pub struct CorsConfiguration {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<HttpMethod>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub allow_private_network: bool,
    pub max_age: Option<u64>,
}

impl CorsConfiguration {
    pub fn new() -> Self {
        Self {
            allowed_origins: Vec::new(),
            allowed_methods: vec![HttpMethod::GET, HttpMethod::HEAD, HttpMethod::POST],
            allowed_headers: Vec::new(),
            exposed_headers: Vec::new(),
            allow_credentials: false,
            allow_private_network: false,
            max_age: None,
        }
    }

    /// Check if the origin is allowed
    pub fn check_origin(&self, request_origin: &str) -> Option<String> {
        if self.allowed_origins.is_empty() {
            return None;
        }

        for allowed_origin in &self.allowed_origins {
            if allowed_origin == "*" || allowed_origin == request_origin {
                return Some(if allowed_origin == "*" && self.allow_credentials {
                    request_origin.to_string()
                } else {
                    allowed_origin.clone()
                });
            }
        }

        None
    }

    /// Check if the HTTP method is allowed
    pub fn check_http_method(&self, request_method: &HttpMethod) -> Option<Vec<HttpMethod>> {
        if self.allowed_methods.contains(request_method) {
            Some(self.allowed_methods.clone())
        } else {
            None
        }
    }

    /// Check if the headers are allowed
    pub fn check_headers(&self, request_headers: &[String]) -> Option<Vec<String>> {
        if self.allowed_headers.is_empty() {
            return None;
        }

        let request_headers_lower: HashSet<String> =
            request_headers.iter().map(|h| h.to_lowercase()).collect();

        let allowed_headers_lower: HashSet<String> = self
            .allowed_headers
            .iter()
            .map(|h| h.to_lowercase())
            .collect();

        if request_headers_lower.is_subset(&allowed_headers_lower) {
            Some(request_headers.to_vec())
        } else {
            None
        }
    }
}

impl Default for CorsConfiguration {
    fn default() -> Self {
        Self::new()
    }
}
