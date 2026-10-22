//! The predicate catalogue and its string DSL parser.

use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use pingora::http::Method;
use pingora::proxy::Session;
use regex::Regex;

use super::cookie_route_predicate_factory::CookieRoutePredicateFactory;
use super::header_route_predicate_factory::HeaderRoutePredicateFactory;
use super::host_route_predicate_factory::HostRoutePredicateFactory;
use super::mehod_route_predicate_factory::MethodRoutePredicateFactory;
use super::path_route_predicate_factory::{build_path_pattern, PathRoutePredicateFactory};
use super::query_route_predicate_factory::QueryRoutePredicateFactory;
use super::remote_addr_route_predicate_factory::RemoteAddrRoutePredicateFactory;
use super::route_predicate::RoutePredicate;
use super::weight_route_predicate_factory::WeightRoutePredicateFactory;
use super::x_forwarded_remote_addr_route_predicate_factory::XForwardedRemoteAddrRoutePredicateFactory;
use super::zoned_datetime_route_predicate_factory::{
    after, before, between, ZonedDateTimeRoutePredicateFactory,
};
use crate::config::ConfigError;
use crate::filter::spec::{unknown_name_error, FilterSpec};
use crate::util::key_value::KeyValue;

/// Every predicate name the parser accepts.
pub const KNOWN_PREDICATES: &[&str] = &[
    "After",
    "Before",
    "Between",
    "Cookie",
    "Header",
    "Host",
    "Method",
    "Path",
    "Query",
    "RemoteAddr",
    "Weight",
    "XForwardedRemoteAddr",
];

/// One route predicate.
#[derive(Debug, Clone)]
pub enum RoutePredicateFactory {
    /// Match on the current time being after a timestamp.
    ZonedDateTimePredicates(ZonedDateTimeRoutePredicateFactory),
    /// Match on a cookie value.
    CookiePredicates(CookieRoutePredicateFactory),
    /// Match on a request header.
    HeaderPredicates(HeaderRoutePredicateFactory),
    /// Match on the request host.
    HostPredicates(HostRoutePredicateFactory),
    /// Match on the request method.
    MethodPredicates(MethodRoutePredicateFactory),
    /// Match on the request path.
    PathPredicates(PathRoutePredicateFactory),
    /// Match on a query parameter.
    QueryPredicates(QueryRoutePredicateFactory),
    /// Match on the peer address.
    RemoteAddrPredicates(RemoteAddrRoutePredicateFactory),
    /// Match by route group weight.
    WeightPredicates(WeightRoutePredicateFactory),
    /// Match on the client address behind a trusted proxy.
    XForwardedRemoteAddr(XForwardedRemoteAddrRoutePredicateFactory),
}

impl RoutePredicateFactory {
    /// Whether the predicate accepts the current request.
    pub fn matches(&self, session: &mut Session) -> bool {
        match self {
            RoutePredicateFactory::ZonedDateTimePredicates(factory) => factory.matches(session),
            RoutePredicateFactory::CookiePredicates(factory) => factory.matches(session),
            RoutePredicateFactory::HeaderPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::HostPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::MethodPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::PathPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::QueryPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::RemoteAddrPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::WeightPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::XForwardedRemoteAddr(factory) => factory.matches(session),
        }
    }
}

impl TryFrom<&str> for RoutePredicateFactory {
    type Error = ConfigError;

    /// Parse a predicate expression, rejecting unknown names and bad arguments.
    fn try_from(raw: &str) -> Result<Self, ConfigError> {
        let spec = FilterSpec::parse(raw)?;
        let name = spec.name.as_str();

        let factory = match name {
            "Before" | "After" | "Between" => {
                let value = match name {
                    "Before" => before(spec.args.join(",").trim()),
                    "After" => after(spec.args.join(",").trim()),
                    "Between" => {
                        let joined = spec.args.join(",");
                        let (start, end) = joined
                            .split_once(',')
                            .or_else(|| joined.split_once('/'))
                            .ok_or_else(|| {
                                ConfigError::new(
                                    "predicates.Between",
                                    "expects two timestamps separated by `,` or `/`",
                                )
                            })?;
                        between(start.trim(), end.trim())
                    }
                    _ => unreachable!(),
                };

                match value {
                    Ok(predicate) => RoutePredicateFactory::ZonedDateTimePredicates(predicate),
                    Err(error) => {
                        return Err(ConfigError::new(
                            format!("predicates.{name}"),
                            error.to_string(),
                        ));
                    }
                }
            }

            "Cookie" => {
                let (cookie_name, pattern) = name_and_pattern(&spec)?;
                RoutePredicateFactory::CookiePredicates(CookieRoutePredicateFactory {
                    name: cookie_name,
                    regex: pattern,
                })
            }

            "Header" => {
                let (header_name, pattern) = name_and_pattern(&spec)?;
                RoutePredicateFactory::HeaderPredicates(HeaderRoutePredicateFactory {
                    header: KeyValue {
                        k: header_name.into(),
                        v: spec.args.get(1).cloned(),
                    },
                    regex: pattern,
                })
            }

            "Host" => {
                let hosts = spec
                    .args
                    .iter()
                    .map(|host| host.trim().to_ascii_lowercase())
                    .filter(|host| !host.is_empty())
                    .collect::<Vec<_>>();

                if hosts.is_empty() {
                    return Err(ConfigError::new(
                        "predicates.Host",
                        "expects at least one host pattern",
                    ));
                }

                RoutePredicateFactory::HostPredicates(HostRoutePredicateFactory { hosts })
            }

            "Method" => {
                let methods = spec
                    .args
                    .iter()
                    .map(|method| {
                        Method::from_bytes(method.trim().as_bytes()).map_err(|_| {
                            ConfigError::new(
                                "predicates.Method",
                                format!("`{}` is not a valid HTTP method", method.trim()),
                            )
                        })
                    })
                    .collect::<Result<HashSet<_>, _>>()?;

                if methods.is_empty() {
                    return Err(ConfigError::new(
                        "predicates.Method",
                        "expects at least one HTTP method",
                    ));
                }

                RoutePredicateFactory::MethodPredicates(MethodRoutePredicateFactory { methods })
            }

            "Path" => {
                let paths = spec
                    .args
                    .iter()
                    .map(|pattern| {
                        build_path_pattern(pattern).ok_or_else(|| {
                            ConfigError::new(
                                "predicates.Path",
                                format!("`{}` is not a valid path pattern", pattern.trim()),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                if paths.is_empty() {
                    return Err(ConfigError::new(
                        "predicates.Path",
                        "expects at least one path pattern",
                    ));
                }

                RoutePredicateFactory::PathPredicates(PathRoutePredicateFactory { paths })
            }

            "Query" => {
                let (parameter_name, pattern) = name_and_pattern(&spec)?;
                RoutePredicateFactory::QueryPredicates(QueryRoutePredicateFactory {
                    name: parameter_name,
                    regex: pattern,
                })
            }

            "RemoteAddr" => {
                let remote_addrs = parse_networks(&spec, "predicates.RemoteAddr")?;
                RoutePredicateFactory::RemoteAddrPredicates(RemoteAddrRoutePredicateFactory {
                    remote_addrs,
                })
            }

            "Weight" => {
                if spec.args.len() != 2 {
                    return Err(ConfigError::new(
                        "predicates.Weight",
                        format!(
                            "`Weight` expects `group, weight`, got {} argument(s)",
                            spec.args.len()
                        ),
                    ));
                }

                let group = spec.args[0].trim();
                if group.is_empty() {
                    return Err(ConfigError::new(
                        "predicates.Weight",
                        "the route group must not be empty",
                    ));
                }

                let weight = spec.args[1].trim().parse::<u32>().map_err(|_| {
                    ConfigError::new(
                        "predicates.Weight",
                        format!("`{}` is not a valid weight", spec.args[1].trim()),
                    )
                })?;

                if weight == 0 {
                    return Err(ConfigError::new(
                        "predicates.Weight",
                        "weight must be greater than zero",
                    ));
                }

                RoutePredicateFactory::WeightPredicates(WeightRoutePredicateFactory::new(
                    group.to_string(),
                    weight,
                ))
            }

            "XForwardedRemoteAddr" => {
                let trusted_networks =
                    parse_networks(&spec, "predicates.XForwardedRemoteAddr")?;
                RoutePredicateFactory::XForwardedRemoteAddr(
                    XForwardedRemoteAddrRoutePredicateFactory { trusted_networks },
                )
            }

            unknown => return Err(unknown_name_error("predicate", unknown, KNOWN_PREDICATES)),
        };

        Ok(factory)
    }
}

/// Split `name[, pattern]` arguments and compile the optional pattern.
fn name_and_pattern(spec: &FilterSpec) -> Result<(String, Option<Regex>), ConfigError> {
    let Some(name) = spec.args.first().map(|name| name.trim()) else {
        return Err(ConfigError::new(
            format!("predicates.{}", spec.name),
            "expects a name",
        ));
    };

    if name.is_empty() {
        return Err(ConfigError::new(
            format!("predicates.{}", spec.name),
            "name must not be empty",
        ));
    }

    let pattern = match spec.args.get(1).map(|pattern| pattern.trim()) {
        Some(pattern) if !pattern.is_empty() => Some(Regex::new(pattern).map_err(|error| {
            ConfigError::new(
                format!("predicates.{}", spec.name),
                format!("invalid regular expression `{pattern}`: {error}"),
            )
        })?),
        _ => None,
    };

    Ok((name.to_string(), pattern))
}

/// Parse every argument as an IP address or CIDR block.
fn parse_networks(spec: &FilterSpec, path: &str) -> Result<Vec<IpNetwork>, ConfigError> {
    if spec.args.is_empty() {
        return Err(ConfigError::new(
            path,
            "expects at least one IP address or CIDR block",
        ));
    }

    spec.args
        .iter()
        .map(|entry| {
            parse_ip_network(entry.trim()).ok_or_else(|| {
                ConfigError::new(
                    path,
                    format!("`{}` is not an IP address or CIDR block", entry.trim()),
                )
            })
        })
        .collect()
}

fn parse_ip_network(source: &str) -> Option<IpNetwork> {
    if let Ok(network) = IpNetwork::from_str(source) {
        return Some(network);
    }

    match IpAddr::from_str(source).ok()? {
        IpAddr::V4(ip) => Ipv4Network::new(ip, 32).ok().map(IpNetwork::V4),
        IpAddr::V6(ip) => Ipv6Network::new(ip, 128).ok().map(IpNetwork::V6),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_ip_network, RoutePredicateFactory};

    #[test]
    fn between_with_missing_boundary_is_an_error() {
        let error =
            RoutePredicateFactory::try_from("Between=2024-01-01T00:00:00+00:00").unwrap_err();
        assert!(error.message.contains("two timestamps"), "{}", error.message);
    }

    #[test]
    fn query_predicate_supports_optional_regex() {
        let predicate = RoutePredicateFactory::try_from("Query=page,^\\d+$").unwrap();
        assert!(matches!(
            predicate,
            RoutePredicateFactory::QueryPredicates(_)
        ));
    }

    #[test]
    fn weight_predicate_parses_group_and_weight() {
        let predicate = RoutePredicateFactory::try_from("Weight=group1, 8").unwrap();
        match predicate {
            RoutePredicateFactory::WeightPredicates(factory) => {
                assert_eq!(factory.group, "group1");
                assert_eq!(factory.weight, 8);
            }
            _ => panic!("expected weight predicate"),
        }
    }

    #[test]
    fn invalid_weight_predicate_is_an_error() {
        assert!(RoutePredicateFactory::try_from("Weight=group1").is_err());
        assert!(RoutePredicateFactory::try_from("Weight=group1,0").is_err());
    }

    #[test]
    fn invalid_query_regex_is_an_error() {
        assert!(RoutePredicateFactory::try_from("Query=page,[").is_err());
    }

    #[test]
    fn unknown_predicate_name_is_an_error_listing_known_names() {
        let error = RoutePredicateFactory::try_from("Pathh=/a").unwrap_err();
        assert!(error.message.contains("Path"), "{}", error.message);
    }

    #[test]
    fn parse_ip_network_accepts_single_ip_and_cidr() {
        assert!(parse_ip_network("10.0.0.1").is_some());
        assert!(parse_ip_network("10.0.0.0/24").is_some());
        assert!(parse_ip_network("invalid").is_none());
    }

    #[test]
    fn remote_addr_rejects_an_invalid_cidr() {
        assert!(RoutePredicateFactory::try_from("RemoteAddr=10.0.0.0/33").is_err());
    }
}
