//! Cross-field validation that serde alone cannot express.
//!
//! Every check returns a [`ConfigError`] carrying a YAML-ish path so the
//! operator can jump straight to the offending line. Validation collects all
//! failures instead of stopping at the first one.

use std::net::SocketAddr;

use ipnetwork::IpNetwork;

use super::{ConfigError, GatewayConfig};

const SCHEMES: [&str; 3] = ["http", "https", "lb"];

/// Run every cross-field check and return all failures, in document order.
pub(crate) fn validate(config: &GatewayConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    validate_bind(
        "server.bind",
        &config.server.bind,
        &mut errors,
    );
    validate_bind(
        "server.admin.bind",
        &config.server.admin.bind,
        &mut errors,
    );

    validate_trusted_proxies(&config.trusted_proxies, &mut errors);
    validate_server_knobs(config, &mut errors);
    validate_breakers(config, &mut errors);
    validate_routes(config, &mut errors);

    errors
}

fn validate_bind(path: &str, value: &str, errors: &mut Vec<ConfigError>) {
    if !is_valid_bind(value) {
        errors.push(ConfigError::new(
            path,
            format!("`{value}` is not a `host:port` listen address"),
        ));
    }
}

/// Whether `value` looks like a `host:port` listen address.
///
/// The command line overrides go through the same check, so `--bind nonsense`
/// is a configuration error instead of a listener panic.
pub fn is_valid_bind(value: &str) -> bool {
    match value.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => port.parse::<u16>().is_ok(),
        _ => false,
    }
}

/// Reject listener knobs that pingora cannot apply.
///
/// Accepting them would leave an operator believing a limit is in force when it
/// is not, which is the failure mode A1 exists to prevent.
fn validate_server_knobs(config: &GatewayConfig, errors: &mut Vec<ConfigError>) {
    if config.server.keepalive.is_some() {
        errors.push(ConfigError::new(
            "server.keepalive",
            "downstream keep-alive is not configurable in this build; remove the key",
        ));
    }

    if config.server.max_connection != 0 {
        errors.push(ConfigError::new(
            "server.max_connection",
            "connection limiting is not implemented in this build; remove the key",
        ));
    }

    // A health check that never runs would hide dead backends behind 502s.
    if config.upstream.health_check.enabled {
        errors.push(ConfigError::new(
            "upstream.health_check.enabled",
            "health checking is not implemented in this build; dead backends surface as \
             502 and as breaker failures. Remove the key or set `enabled: false`",
        ));
    }

    // TLS is unavailable, so a verification bypass would only be theatre.
    if config.upstream.tls.insecure_skip_verify {
        errors.push(ConfigError::new(
            "upstream.tls.insecure_skip_verify",
            "TLS upstreams are unavailable in this build, so there is nothing to skip; \
             remove the key",
        ));
    }
}

fn validate_trusted_proxies(entries: &[String], errors: &mut Vec<ConfigError>) {
    for (index, entry) in entries.iter().enumerate() {
        if entry.parse::<IpNetwork>().is_err() {
            errors.push(ConfigError::new(
                format!("trusted_proxies[{index}]"),
                format!("`{entry}` is not an IP address or CIDR block"),
            ));
        }
    }
}

fn validate_breakers(config: &GatewayConfig, errors: &mut Vec<ConfigError>) {
    for (id, breaker) in &config.breakers {
        if breaker.failure_threshold == 0 {
            errors.push(ConfigError::new(
                format!("breakers.{id}.failure_threshold"),
                "must be greater than zero",
            ));
        }
        if breaker.half_open_permits == 0 {
            errors.push(ConfigError::new(
                format!("breakers.{id}.half_open_permits"),
                "must be greater than zero",
            ));
        }
        if breaker.wait_duration_in_open_state.is_zero() {
            errors.push(ConfigError::new(
                format!("breakers.{id}.wait_duration_in_open_state"),
                "must be greater than zero",
            ));
        }
    }
}

fn validate_routes(config: &GatewayConfig, errors: &mut Vec<ConfigError>) {
    let mut seen_ids: Vec<&str> = Vec::with_capacity(config.routes.len());

    for (index, route) in config.routes.iter().enumerate() {
        let path = |field: &str| format!("routes[{index}].{field}");

        if route.id.trim().is_empty() {
            errors.push(ConfigError::new(path("id"), "route id must not be empty"));
        } else if seen_ids.contains(&route.id.as_str()) {
            errors.push(ConfigError::new(
                path("id"),
                format!("duplicate route id `{}`", route.id),
            ));
        } else {
            seen_ids.push(&route.id);
        }

        let scheme = validate_uri(&route.uri, &path("uri"), errors);

        match scheme.as_deref() {
            Some("lb") if route.upstreams.is_empty() => {
                errors.push(ConfigError::new(
                    path("upstreams"),
                    "an `lb://` route requires at least one upstream",
                ));
            }
            Some("http" | "https") if !route.upstreams.is_empty() => {
                errors.push(ConfigError::new(
                    path("upstreams"),
                    "`upstreams` is only valid on `lb://` routes",
                ));
            }
            _ => {}
        }

        for (upstream_index, upstream) in route.upstreams.iter().enumerate() {
            let upstream_path = path(&format!("upstreams[{upstream_index}]"));
            if upstream.addr.parse::<SocketAddr>().is_err() {
                errors.push(ConfigError::new(
                    format!("{upstream_path}.addr"),
                    format!("`{}` is not a `IP:port` backend address", upstream.addr),
                ));
            }
            if upstream.weight == 0 {
                errors.push(ConfigError::new(
                    format!("{upstream_path}.weight"),
                    "must be greater than zero",
                ));
            }
        }
    }
}

/// Validate the `uri` shape and return its scheme when it is well formed.
fn validate_uri(uri: &str, path: &str, errors: &mut Vec<ConfigError>) -> Option<String> {
    let Some((scheme, rest)) = uri.split_once("://") else {
        errors.push(ConfigError::new(
            path,
            format!("`{uri}` has no scheme, expected one of {}", SCHEMES.join(", ")),
        ));
        return None;
    };

    let scheme = scheme.to_ascii_lowercase();
    if !SCHEMES.contains(&scheme.as_str()) {
        errors.push(ConfigError::new(
            path,
            format!(
                "unsupported scheme `{scheme}`, expected one of {}",
                SCHEMES.join(", ")
            ),
        ));
        return None;
    }

    // This build of pingora carries no TLS backend, so an `https://` route
    // could only ever answer 502. Fail at startup instead (A1).
    if scheme == "https" {
        errors.push(ConfigError::new(
            path,
            "TLS upstreams are unavailable: this build of pingora has no TLS \
             backend compiled in, so an `https://` route can never connect. \
             Use `http://` or `lb://`, or rebuild with pingora's `openssl`, \
             `boringssl` or `rustls` feature and remove this guard",
        ));
        return None;
    }

    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        errors.push(ConfigError::new(path, "uri has an empty authority"));
        return None;
    }

    // `https://:443` has an authority but no host, which would produce an empty
    // SNI and disable TLS verification downstream.
    let host_part = authority
        .rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(authority);
    if host_part.is_empty() || host_part == "[]" {
        errors.push(ConfigError::new(
            path,
            format!("`{authority}` has no host name"),
        ));
        return None;
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        let bracketed_ipv6 = host.starts_with('[') && host.ends_with(']');
        let looks_like_port = !port.is_empty();
        if !looks_like_port || (!bracketed_ipv6 && host.contains(':')) {
            errors.push(ConfigError::new(
                path,
                format!("`{authority}` is not a valid authority"),
            ));
            return None;
        }
        if port.parse::<u16>().is_err() {
            errors.push(ConfigError::new(
                path,
                format!("`{port}` is not a valid port"),
            ));
            return None;
        }
    }

    Some(scheme)
}
