use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use pingora::http::Method;
use pingora::proxy::Session;
use regex::Regex;
use tracing::warn;

use super::zoned_datetime_route_predicate_factory::{after, before, between};
use super::XForwardedRemoteAddrRoutePredicateFactory;
use crate::util::str::StrUtil;

use super::{
    cookie_route_predicate_factory::CookieRoutePredicateFactory,
    header_route_predicate_factory::HeaderRoutePredicateFactory,
    host_route_predicate_factory::HostRoutePredicateFactory,
    mehod_route_predicate_factory::MethodRoutePredicateFactory,
    path_route_predicate_factory::{build_path_pattern, PathRoutePredicateFactory},
    query_route_predicate_factory::QueryRoutePredicateFactory,
    remote_addr_route_predicate_factory::RemoteAddrRoutePredicateFactory,
    route_predicate::RoutePredicate,
    weight_route_predicate_factory::WeightRoutePredicateFactory,
    zoned_datetime_route_predicate_factory::ZonedDateTimeRoutePredicateFactory,
};

#[derive(Debug, Clone)]
pub enum RoutePredicateFactory {
    ZonedDateTimePredicates(ZonedDateTimeRoutePredicateFactory),
    CookiePredicates(CookieRoutePredicateFactory),
    HeaderPredicates(HeaderRoutePredicateFactory),
    HostPredicates(HostRoutePredicateFactory),
    MethodPredicates(MethodRoutePredicateFactory),
    PathPredicates(PathRoutePredicateFactory),
    QueryPredicates(QueryRoutePredicateFactory),
    RemoteAddrPredicates(RemoteAddrRoutePredicateFactory),
    WeightPredicates(WeightRoutePredicateFactory),
    XForwardedRemoteAddr(XForwardedRemoteAddrRoutePredicateFactory),
    Nothing,
}

impl RoutePredicateFactory {
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
            RoutePredicateFactory::Nothing => false,
        }
    }
}

impl Into<RoutePredicateFactory> for &String {
    fn into(self) -> RoutePredicateFactory {
        let (key, value) = match self.split_once('=') {
            Some((k, v)) => (k.trim(), v),
            None => return RoutePredicateFactory::Nothing,
        };

        if key.is_empty() {
            return RoutePredicateFactory::Nothing;
        }

        match key {
            "Before" | "After" | "Between" => {
                let result = match key {
                    "Before" => before(value),
                    "After" => after(value),
                    "Between" => {
                        let Some((start, end)) = value.trim().split_once(',') else {
                            warn!("Between predicate requires two timestamps separated by comma");
                            return RoutePredicateFactory::Nothing;
                        };
                        between(start.trim(), end.trim())
                    }
                    _ => unreachable!(),
                };

                match result {
                    Ok(pred) => RoutePredicateFactory::ZonedDateTimePredicates(pred),
                    Err(e) => {
                        warn!("Time predicate error: {e}");
                        RoutePredicateFactory::Nothing
                    }
                }
            }

            "Cookie" => {
                let cookie = StrUtil::parse_kv_one_and_option(value);
                if cookie.k.trim().is_empty() {
                    warn!("Cookie predicate requires a cookie name");
                    return RoutePredicateFactory::Nothing;
                }

                let regex = match compile_optional_regex(cookie.v.as_deref(), "Cookie") {
                    Ok(regex) => regex,
                    Err(()) => return RoutePredicateFactory::Nothing,
                };
                RoutePredicateFactory::CookiePredicates(CookieRoutePredicateFactory {
                    name: cookie.k.trim().to_string(),
                    regex,
                })
            }

            "Header" => {
                let header = StrUtil::parse_kv_one_and_option(value);
                if header.k.trim().is_empty() {
                    warn!("Header predicate requires a header name");
                    return RoutePredicateFactory::Nothing;
                }

                let regex = match compile_optional_regex(header.v.as_deref(), "Header") {
                    Ok(regex) => regex,
                    Err(()) => return RoutePredicateFactory::Nothing,
                };
                RoutePredicateFactory::HeaderPredicates(HeaderRoutePredicateFactory {
                    header,
                    regex,
                })
            }

            "Host" => {
                let hosts = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_ascii_lowercase())
                    .collect::<Vec<_>>();

                if hosts.is_empty() {
                    warn!("Host predicate requires at least one host pattern");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::HostPredicates(HostRoutePredicateFactory { hosts })
            }

            "Method" => {
                let methods = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        Method::from_bytes(s.as_bytes())
                            .map_err(|_| format!("Invalid HTTP method: {s}"))
                    })
                    .collect::<Result<HashSet<_>, _>>();

                match methods {
                    Ok(methods) if !methods.is_empty() => {
                        RoutePredicateFactory::MethodPredicates(MethodRoutePredicateFactory {
                            methods,
                        })
                    }
                    Ok(_) => {
                        warn!("Method predicate requires at least one method");
                        RoutePredicateFactory::Nothing
                    }
                    Err(e) => {
                        warn!("{e}");
                        RoutePredicateFactory::Nothing
                    }
                }
            }

            "Path" => {
                let paths = value
                    .split(',')
                    .map(str::trim)
                    .filter_map(build_path_pattern)
                    .collect::<Vec<_>>();

                if paths.is_empty() {
                    warn!("Path predicate requires at least one valid path pattern");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::PathPredicates(PathRoutePredicateFactory { paths })
            }

            "Query" => {
                let query = StrUtil::parse_kv_one_and_option(value);
                if query.k.trim().is_empty() {
                    warn!("Query predicate requires a parameter name");
                    return RoutePredicateFactory::Nothing;
                }

                let regex = match compile_optional_regex(query.v.as_deref(), "Query") {
                    Ok(regex) => regex,
                    Err(()) => return RoutePredicateFactory::Nothing,
                };
                RoutePredicateFactory::QueryPredicates(QueryRoutePredicateFactory {
                    name: query.k.trim().to_string(),
                    regex,
                })
            }

            "RemoteAddr" => {
                let remote_addrs = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .filter_map(parse_ip_network)
                    .collect::<Vec<_>>();

                if remote_addrs.is_empty() {
                    warn!("RemoteAddr predicate requires at least one valid IP or CIDR");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::RemoteAddrPredicates(RemoteAddrRoutePredicateFactory {
                    remote_addrs,
                })
            }

            "Weight" => {
                let Some((group, weight)) = value.split_once(',') else {
                    warn!("Weight predicate requires group and weight");
                    return RoutePredicateFactory::Nothing;
                };

                let group = group.trim();
                if group.is_empty() {
                    warn!("Weight predicate requires a non-empty group");
                    return RoutePredicateFactory::Nothing;
                }

                let Ok(weight) = weight.trim().parse::<u32>() else {
                    warn!("Weight predicate requires a valid integer weight");
                    return RoutePredicateFactory::Nothing;
                };

                if weight == 0 {
                    warn!("Weight predicate requires weight > 0");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::WeightPredicates(WeightRoutePredicateFactory::new(
                    group.to_string(),
                    weight,
                ))
            }

            "XForwardedRemoteAddr" => {
                let trusted_networks = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .filter_map(parse_ip_network)
                    .collect::<Vec<_>>();

                if trusted_networks.is_empty() {
                    warn!("XForwardedRemoteAddr predicate requires at least one valid IP or CIDR");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::XForwardedRemoteAddr(
                    XForwardedRemoteAddrRoutePredicateFactory { trusted_networks },
                )
            }

            _ => {
                warn!("Unsupported predicate: {key}");
                RoutePredicateFactory::Nothing
            }
        }
    }
}

fn compile_optional_regex(
    pattern: Option<&str>,
    predicate_name: &str,
) -> Result<Option<Regex>, ()> {
    let Some(pattern) = pattern.map(str::trim).filter(|pattern| !pattern.is_empty()) else {
        return Ok(None);
    };

    match Regex::new(pattern) {
        Ok(regex) => Ok(Some(regex)),
        Err(error) => {
            warn!("Invalid regex in {predicate_name} predicate: {error}");
            Err(())
        }
    }
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
    fn between_with_missing_boundary_returns_nothing() {
        let predicate = (&"Between=2024-01-01T00:00:00+00:00".to_string()).into();
        assert!(matches!(predicate, RoutePredicateFactory::Nothing));
    }

    #[test]
    fn query_predicate_supports_optional_regex() {
        let predicate = (&"Query=page,^\\d+$".to_string()).into();
        assert!(matches!(
            predicate,
            RoutePredicateFactory::QueryPredicates(_)
        ));
    }

    #[test]
    fn weight_predicate_parses_group_and_weight() {
        let predicate = (&"Weight=group1, 8".to_string()).into();
        match predicate {
            RoutePredicateFactory::WeightPredicates(factory) => {
                assert_eq!(factory.group, "group1");
                assert_eq!(factory.weight, 8);
            }
            _ => panic!("expected weight predicate"),
        }
    }

    #[test]
    fn invalid_weight_predicate_returns_nothing() {
        let missing_weight = (&"Weight=group1".to_string()).into();
        let zero_weight = (&"Weight=group1,0".to_string()).into();

        assert!(matches!(missing_weight, RoutePredicateFactory::Nothing));
        assert!(matches!(zero_weight, RoutePredicateFactory::Nothing));
    }

    #[test]
    fn invalid_query_regex_returns_nothing() {
        let predicate = (&"Query=page,[".to_string()).into();
        assert!(matches!(predicate, RoutePredicateFactory::Nothing));
    }

    #[test]
    fn parse_ip_network_accepts_single_ip_and_cidr() {
        assert!(parse_ip_network("10.0.0.1").is_some());
        assert!(parse_ip_network("10.0.0.0/24").is_some());
        assert!(parse_ip_network("invalid").is_none());
    }
}
