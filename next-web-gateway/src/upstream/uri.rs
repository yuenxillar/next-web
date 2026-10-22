//! Parsing of route `uri` values into scheme, authority and path prefix.

use crate::config::ConfigError;

/// How a route reaches its upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamScheme {
    /// Plain HTTP to a fixed authority.
    Http,
    /// TLS to a fixed authority.
    Https,
    /// Load balanced across the route's upstreams.
    Lb,
}

impl UpstreamScheme {
    /// Whether the scheme carries TLS.
    pub fn is_tls(self) -> bool {
        matches!(self, Self::Https)
    }
}

/// A parsed route target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamUri {
    /// Scheme of the target.
    pub scheme: UpstreamScheme,
    /// Host name or IP, without port and without brackets.
    pub host: String,
    /// Port, defaulted from the scheme when absent.
    pub port: u16,
    /// Path prefix carried by the uri, used by later phases.
    pub path_prefix: String,
}

impl UpstreamUri {
    /// Parse `http://host:port`, `https://host:port` or `lb://name`.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let (scheme, rest) = raw.trim().split_once("://").ok_or_else(|| {
            ConfigError::new(
                "routes.uri",
                format!("`{raw}` has no scheme, expected http, https or lb"),
            )
        })?;

        let scheme = match scheme.to_ascii_lowercase().as_str() {
            "http" => UpstreamScheme::Http,
            "https" => UpstreamScheme::Https,
            "lb" => UpstreamScheme::Lb,
            other => {
                return Err(ConfigError::new(
                    "routes.uri",
                    format!("unsupported scheme `{other}`, expected http, https or lb"),
                ));
            }
        };

        let (authority, path_prefix) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, String::new()),
        };

        if authority.is_empty() {
            return Err(ConfigError::new("routes.uri", "uri has an empty authority"));
        }

        let (host, port) = split_authority(authority)?;

        Ok(Self {
            scheme,
            host,
            port,
            path_prefix,
        })
    }

    /// The authority as it should appear in a `Host` header.
    pub fn authority(&self) -> String {
        match (self.host.contains(':'), self.port) {
            (true, port) => format!("[{}]:{port}", self.host),
            (false, port) => format!("{}:{port}", self.host),
        }
    }

    /// The `host:port` shown in logs and metrics.
    pub fn endpoint(&self) -> String {
        self.authority()
    }
}

/// Split `host:port`, tolerating bracketed IPv6 and a missing port.
fn split_authority(authority: &str) -> Result<(String, u16), ConfigError> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']').ok_or_else(|| {
            ConfigError::new("routes.uri", format!("`{authority}` has an unclosed bracket"))
        })?;

        let port = match tail.strip_prefix(':') {
            Some(port) => parse_port(port)?,
            None => return Ok((host.to_string(), 0)),
        };

        return Ok((host.to_string(), port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => Ok((host.to_string(), parse_port(port)?)),
        // A bare IPv6 address without brackets, or a host without a port.
        Some(_) => Err(ConfigError::new(
            "routes.uri",
            format!("`{authority}` is not a valid authority"),
        )),
        None => Ok((authority.to_string(), 0)),
    }
}

fn parse_port(port: &str) -> Result<u16, ConfigError> {
    port.parse::<u16>().map_err(|_| {
        ConfigError::new("routes.uri", format!("`{port}` is not a valid port"))
    })
}

#[cfg(test)]
mod tests {
    use super::{UpstreamScheme, UpstreamUri};

    #[test]
    fn parses_http_https_and_lb_uris() {
        let http = UpstreamUri::parse("http://backend:8080").unwrap();
        assert_eq!(http.scheme, UpstreamScheme::Http);
        assert_eq!(http.host, "backend");
        assert_eq!(http.port, 8080);

        let https = UpstreamUri::parse("https://api.example.com").unwrap();
        assert_eq!(https.scheme, UpstreamScheme::Https);
        assert_eq!(https.host, "api.example.com");

        let lb = UpstreamUri::parse("lb://user-service").unwrap();
        assert_eq!(lb.scheme, UpstreamScheme::Lb);
        assert_eq!(lb.host, "user-service");
    }

    #[test]
    fn rejects_uri_without_scheme_or_with_unknown_scheme() {
        assert!(UpstreamUri::parse("nope").is_err());
        assert!(UpstreamUri::parse("ftp://host:21").is_err());
        assert!(UpstreamUri::parse("http://").is_err());
    }

    #[test]
    fn keeps_the_path_prefix_but_not_the_query() {
        let uri = UpstreamUri::parse("http://host:80/api/v1").unwrap();
        assert_eq!(uri.path_prefix, "/api/v1");
    }

    #[test]
    fn parses_bracketed_ipv6() {
        let uri = UpstreamUri::parse("http://[2001:db8::1]:8080").unwrap();
        assert_eq!(uri.host, "2001:db8::1");
        assert_eq!(uri.port, 8080);
        assert_eq!(uri.authority(), "[2001:db8::1]:8080");
    }

    #[test]
    fn rejects_an_invalid_port() {
        assert!(UpstreamUri::parse("http://host:notaport").is_err());
    }
}
