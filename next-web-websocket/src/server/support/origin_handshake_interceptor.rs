//! Origin Handshake Interceptor for WebSocket connections
//!
//! This module provides CORS origin checking for WebSocket handshake requests,
//! equivalent to Spring's OriginHandshakeInterceptor.
use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use axum::http::{StatusCode, Uri, uri::Authority};
use next_web_core::anys::any_value::AnyValue;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::traits::http::http_request::HttpRequest;
use next_web_core::traits::http::http_response::HttpResponse;
use tracing::{debug, info};

use crate::server::handshake_interceptor::HandshakeInterceptor;

/// Configuration for CORS origin validation
#[derive(Debug, Clone, Default)]
pub struct CorsConfiguration {
    allowed_origins: HashSet<String>,
    allowed_origin_patterns: HashSet<String>,
}

impl CorsConfiguration {
    /// Create a new CORS configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allowed origins (exact match)
    pub fn set_allowed_origins<I, T>(mut self, origins: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.allowed_origins = origins
            .into_iter()
            .map(|s| s.into())
            .collect::<HashSet<String>>();
        self
    }

    /// Set allowed origin patterns (supports `*` wildcards in the full origin string)
    pub fn set_allowed_origin_patterns<I, T>(mut self, patterns: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        self.allowed_origin_patterns = patterns
            .into_iter()
            .map(|s| s.into())
            .collect::<HashSet<String>>();
        self
    }

    /// Check if the origin is allowed
    /// Returns `true` if allowed, `false` otherwise
    pub fn check_origin(&self, origin: Option<&str>) -> bool {
        let Some(origin) = origin else {
            return true;
        };

        if self.allowed_origins.contains(origin) {
            return true;
        }

        self.allowed_origin_patterns
            .iter()
            .any(|pattern| wildcard_match(origin, pattern))
    }
}

/// Origin handshake interceptor for WebSocket connections
#[derive(Debug, Clone)]
pub struct OriginHandshakeInterceptor {
    cors_configuration: CorsConfiguration,
}

impl OriginHandshakeInterceptor {
    /// Create a new interceptor with default configuration (same origin only)
    pub fn new<I, T>(allowed_origins: I, allowed_origin_patterns: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let cors_configuration = CorsConfiguration::default()
            .set_allowed_origins(allowed_origins)
            .set_allowed_origin_patterns(allowed_origin_patterns);

        Self { cors_configuration }
    }

    /// Create an interceptor with allowed origins
    pub fn with_allowed_origins(origins: impl IntoIterator<Item = String>) -> Self {
        let config = CorsConfiguration::new().set_allowed_origins(origins);
        Self {
            cors_configuration: config,
        }
    }

    /// Create an interceptor with allowed origin patterns
    pub fn with_allowed_origin_patterns(patterns: impl IntoIterator<Item = String>) -> Self {
        let config = CorsConfiguration::new().set_allowed_origin_patterns(patterns);
        Self {
            cors_configuration: config,
        }
    }

    /// Create an interceptor with full configuration
    pub fn with_configuration(config: CorsConfiguration) -> Self {
        Self {
            cors_configuration: config,
        }
    }

    /// Check if the origin is allowed (same-origin check + CORS)
    fn is_origin_allowed(&self, request: &dyn HttpRequest) -> bool {
        let origin = request.header("origin");

        if self.is_same_origin(request) || self.cors_configuration.check_origin(origin) {
            return true;
        }

        debug!(
            "Handshake request rejected, Origin header value {:?} not allowed",
            origin
        );
        false
    }

    /// Check if the request is from the same origin
    fn is_same_origin(&self, request: &dyn HttpRequest) -> bool {
        let Some(origin) = request.header("origin") else {
            return true;
        };

        let Some(request_origin) = resolve_request_origin(request) else {
            return false;
        };

        let Some(origin_value) = parse_origin(origin) else {
            return false;
        };

        request_origin == origin_value
    }

    /// Get the CORS configuration (for testing)
    pub fn cors_configuration(&self) -> &CorsConfiguration {
        &self.cors_configuration
    }
}

#[async_trait]
impl HandshakeInterceptor for OriginHandshakeInterceptor {
    async fn before_handshake(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        _attributes: &mut HashMap<String, AnyValue>,
    ) -> Result<bool, BoxError> {
        let origin = request.header("origin");
        if !self.is_origin_allowed(request) {
            response.set_status_code(StatusCode::FORBIDDEN);
            info!(
                "Handshake request rejected, Origin header value {:?} not allowed",
                origin
            );
            return Ok(false);
        }

        Ok(true)
    }
}

fn resolve_request_origin(request: &dyn HttpRequest) -> Option<(String, String, u16)> {
    let scheme = canonical_scheme(
        first_forwarded_value(request.header("x-forwarded-proto"))
            .or_else(|| request.scheme())
            .unwrap_or_else(|| if request.is_secure() { "https" } else { "http" }),
    );

    let forwarded_host = first_forwarded_value(request.header("x-forwarded-host"));
    let authority = forwarded_host
        .and_then(parse_authority)
        .or_else(|| request.header("host").and_then(parse_authority));

    let host = authority
        .as_ref()
        .map(|authority| authority.host().to_ascii_lowercase())
        .or_else(|| request.server_name().map(|host| host.to_ascii_lowercase()))?;

    let port = first_forwarded_value(request.header("x-forwarded-port"))
        .and_then(|value| value.parse::<u16>().ok())
        .or_else(|| authority.as_ref().and_then(Authority::port_u16))
        .or_else(|| request.server_port())
        .unwrap_or_else(|| default_port(scheme));

    Some((scheme.to_string(), host, port))
}

fn parse_origin(origin: &str) -> Option<(String, String, u16)> {
    let origin_uri = origin.parse::<Uri>().ok()?;
    let scheme = canonical_scheme(origin_uri.scheme_str()?);
    let host = origin_uri.host()?.to_ascii_lowercase();
    let port = origin_uri
        .port_u16()
        .unwrap_or_else(|| default_port(scheme));

    Some((scheme.to_string(), host, port))
}

fn canonical_scheme(scheme: &str) -> &str {
    match scheme {
        "ws" => "http",
        "wss" => "https",
        other => other,
    }
}

fn default_port(scheme: &str) -> u16 {
    match canonical_scheme(scheme) {
        "https" => 443,
        _ => 80,
    }
}

fn parse_authority(value: &str) -> Option<Authority> {
    let authority = value.trim();
    if authority.is_empty() {
        return None;
    }

    authority.parse::<Authority>().ok()
}

fn first_forwarded_value(value: Option<&str>) -> Option<&str> {
    value
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return value == pattern;
    }

    let mut remainder = value;
    let mut index = 0usize;

    if !pattern.starts_with('*') {
        let first = parts[0];
        if !remainder.starts_with(first) {
            return false;
        }
        remainder = &remainder[first.len()..];
        index = 1;
    }

    let last_index = parts.len().saturating_sub(1);
    while index < last_index {
        let part = parts[index];
        if !part.is_empty() {
            let Some(position) = remainder.find(part) else {
                return false;
            };
            remainder = &remainder[position + part.len()..];
        }
        index += 1;
    }

    let last = parts[last_index];
    if pattern.ends_with('*') {
        remainder.contains(last) || last.is_empty()
    } else {
        remainder.ends_with(last)
    }
}

#[cfg(test)]
mod tests {
    use super::CorsConfiguration;

    use axum::body::Body;
    use axum::extract::Request;

    fn build_request(headers: &[(&str, &str)]) -> Request {
        let mut builder = Request::builder().uri("/ws");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Body::empty()).expect("request should build")
    }

    #[test]
    fn wildcard_origin_pattern_matches_full_origin_string() {
        let config = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["https://*.example.com".to_string()]);

        assert!(config.check_origin(Some("https://api.example.com")));
        assert!(!config.check_origin(Some("https://api.example.org")));
    }
}
