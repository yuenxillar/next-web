//! Origin Handshake Interceptor for WebSocket connections
//!
//! This module provides CORS origin checking for WebSocket handshake requests,
//! equivalent to Next OriginHandshakeInterceptor.
use std::collections::{HashMap, HashSet};

use axum::http::{StatusCode, Uri, uri::Authority};
use next_web_core::any::AnyValue;
use next_web_core::async_trait;
use next_web_core::error::BoxError;
use next_web_core::http::HttpRequest;
use next_web_core::http::HttpResponse;
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

        if self.allowed_origins.is_empty() && self.allowed_origin_patterns.is_empty() {
            return false;
        }

        if self.allowed_origins.contains(origin) {
            return true;
        }

        // Then the same comparison in a normalised shape. Comparing raw strings
        // rejects origins that name the same place, because the host is
        // case-insensitive and the port is implied by the scheme: a configured
        // `https://example.com` should accept a header of `https://EXAMPLE.com`
        // or `https://example.com:443`.
        if let Some(origin) = parse_origin(origin)
            && self
                .allowed_origins
                .iter()
                .any(|allowed| parse_origin(allowed).is_some_and(|allowed| allowed == origin))
        {
            return true;
        }

        self.allowed_origin_patterns
            .iter()
            .any(|pattern| origin_pattern_matches(origin, pattern))
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

/// Matches an `Origin` against a configured pattern such as
/// `https://*.example.com` or `http://localhost:*`.
///
/// The wildcard is applied to the **host**, never to the whole origin. Matching
/// the whole string, as this used to, let `https://*.example.com` be satisfied
/// by `https://evil.com/api.example.com` — the `.example.com` appeared in the
/// path, not in the host, so the pattern admitted an origin it was never meant
/// to. The scheme and port are compared as whole components.
///
/// A pattern is `[scheme://]host[:port]`, where the scheme may be `*` or absent
/// to accept any, the host may contain `*`, and the port may be `*` or absent to
/// accept any. A bare `*` accepts everything.
fn origin_pattern_matches(origin: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // An origin that does not parse has no host to match on, so it can only be
    // named literally.
    let Some((origin_scheme, origin_host, origin_port)) = parse_origin(origin) else {
        return origin == pattern;
    };

    let (pattern_scheme, pattern_authority) = match pattern.split_once("://") {
        Some((scheme, authority)) => (scheme, authority),
        None => ("*", pattern),
    };
    if pattern_scheme != "*" && canonical_scheme(pattern_scheme) != origin_scheme.as_str() {
        return false;
    }

    let (pattern_host, pattern_port) = match pattern_authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (pattern_authority, None),
    };
    if let Some(pattern_port) = pattern_port
        && pattern_port != "*"
    {
        let Ok(pattern_port) = pattern_port.parse::<u16>() else {
            return false;
        };
        if pattern_port != origin_port {
            return false;
        }
    }

    wildcard_host_match(&origin_host, pattern_host)
}

/// Matches a host against a pattern in which `*` stands for any run of
/// characters. Only ever called with a bare host, and a pattern's `*:*` reaches
/// here as the host `*` because the port is split off first.
fn wildcard_host_match(value: &str, pattern: &str) -> bool {
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

    #[test]
    fn wildcard_origin_pattern_matches_full_origin_string() {
        let config = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["https://*.example.com".to_string()]);

        assert!(config.check_origin(Some("https://api.example.com")));
        assert!(!config.check_origin(Some("https://api.example.org")));
    }

    #[test]
    fn a_wildcard_pattern_matches_the_host_and_nothing_else() {
        let config = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["https://*.example.com".to_string()]);

        // The suffix appears in the path, not in the host, so this is not the
        // subdomain the pattern was written for. Matching the whole origin string
        // used to accept all three of these.
        assert!(!config.check_origin(Some("https://evil.com/api.example.com")));
        assert!(!config.check_origin(Some("https://evil.com?x=.example.com")));
        assert!(!config.check_origin(Some("https://evil.com#.example.com")));

        // A deeper subdomain is still allowed, and case does not matter.
        assert!(config.check_origin(Some("https://a.b.example.com")));
        assert!(config.check_origin(Some("https://API.example.com")));
    }

    #[test]
    fn the_scheme_and_the_port_are_matched_as_whole_components() {
        let config = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["https://*.example.com".to_string()]);

        assert!(!config.check_origin(Some("http://api.example.com")));
        // No port in the pattern accepts any port on an allowed host.
        assert!(config.check_origin(Some("https://api.example.com:8443")));
        assert!(!config.check_origin(Some("https://api.example.com.evil.com")));

        let with_port = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["https://*.example.com:8443".to_string()]);
        assert!(with_port.check_origin(Some("https://api.example.com:8443")));
        assert!(!with_port.check_origin(Some("https://api.example.com:9443")));

        let any_port = CorsConfiguration::new()
            .set_allowed_origin_patterns(vec!["http://localhost:*".to_string()]);
        assert!(any_port.check_origin(Some("http://localhost:3000")));
        assert!(!any_port.check_origin(Some("http://localhost.evil.com:3000")));
    }

    #[test]
    fn a_port_wildcard_does_not_widen_the_host() {
        // `*:*` used to match any origin containing a colon, because the trailing
        // wildcard was satisfied by an empty remainder. It is a deliberate
        // allow-everything pattern, and a bare `*` says that more clearly.
        let universal = CorsConfiguration::new().set_allowed_origin_patterns(vec!["*".to_string()]);
        assert!(universal.check_origin(Some("https://evil.com:9999")));

        let host_only =
            CorsConfiguration::new().set_allowed_origin_patterns(vec!["https://*".to_string()]);
        assert!(host_only.check_origin(Some("https://anything.test")));
        assert!(!host_only.check_origin(Some("http://anything.test")));
    }
}
