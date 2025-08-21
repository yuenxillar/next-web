use std::collections::HashSet;

use pingora::http::Method;
use pingora::proxy::Session;
use regex::Regex;
use tracing::warn;

use crate::route::x_forwarded_remote_addr_route_predicate_factory::XForwardedRemoteAddrRoutePredicateFactory;
use crate::route::zoned_datetime_route_predicate_factory::after;
use crate::route::zoned_datetime_route_predicate_factory::before;
use crate::route::zoned_datetime_route_predicate_factory::between;
use crate::util::str_util::StrUtil;

use super::{
    cookie_route_predicate_factory::CookieRoutePredicateFactory,
    header_route_predicate_factory::HeaderRoutePredicateFactory,
    host_route_predicate_factory::HostRoutePredicateFactory,
    mehod_route_predicate_factory::MethodRoutePredicateFactory,
    path_route_predicate_factory::PathRoutePredicateFactory,
    query_route_predicate_factory::QueryRoutePredicateFactory,
    remote_addr_route_predicate_factory::RemoteAddrRoutePredicateFactory,
    route_predicate::RoutePredicate,
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
            RoutePredicateFactory::XForwardedRemoteAddr(factory) => factory.matches(session),
            RoutePredicateFactory::Nothing => false,
        }
    }
}

impl Into<RoutePredicateFactory> for &String {
    fn into(self) -> RoutePredicateFactory {
        // 1. 拆分 key=value
        let (key, value) = match self.split_once('=') {
            Some((k, v)) => (k.trim(), v),
            None => return RoutePredicateFactory::Nothing,
        };

        if key.is_empty() {
            return RoutePredicateFactory::Nothing;
        }

        match key {
            // =============================
            // 时间谓词: Before, After, Between
            // =============================
            "Before" | "After" | "Between" => {
                let result = match key {
                    "Before" => before(value),
                    "After" => after(value),
                    "Between" => {
                        let (start, end) = value
                            .trim()
                            .split_once(',')
                            .ok_or("Between predicate requires two timestamps separated by comma")
                            .unwrap();
                        between(start.trim(), end.trim())
                    }
                    _ => unreachable!(),
                };

                match result {
                    Ok(pred) => RoutePredicateFactory::ZonedDateTimePredicates(pred),
                    Err(e) => {
                        warn!("Time predicate error: {}", e);
                        RoutePredicateFactory::Nothing
                    }
                }
            }

            // =============================
            // Cookie 谓词
            // =============================
            "Cookie" => {
                let cookies = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                RoutePredicateFactory::CookiePredicates(CookieRoutePredicateFactory { cookies })
            }

            // =============================
            // Header 谓词
            // =============================
            "Header" => {
                let header = StrUtil::parse_kv_one_and_option(value);
                let regex = header
                    .v
                    .as_ref()
                    .map(|pattern| Regex::new(pattern))
                    .transpose()
                    .unwrap_or_else(|e| {
                        warn!("Invalid regex in Header predicate: {}", e);
                        None
                    });

                RoutePredicateFactory::HeaderPredicates(HeaderRoutePredicateFactory {
                    header,
                    regex,
                })
            }

            // =============================
            // Host 谓词
            // =============================
            "Host" => {
                let hosts = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                RoutePredicateFactory::HostPredicates(HostRoutePredicateFactory { hosts })
            }

            // =============================
            // Method 谓词
            // =============================
            "Method" => {
                let methods = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| {
                        Method::from_bytes(s.as_bytes())
                            .map_err(|_| format!("Invalid HTTP method: {}", s))
                    })
                    .collect::<Result<HashSet<_>, _>>();

                match methods {
                    Ok(methods) => {
                        RoutePredicateFactory::MethodPredicates(MethodRoutePredicateFactory {
                            methods,
                        })
                    }
                    Err(e) => {
                        warn!("{}", e);
                        RoutePredicateFactory::Nothing
                    }
                }
            }

            // =============================
            // Path 谓词
            // =============================
            "Path" => {
                let mut paths = matchit::Router::new();
                // 支持多个 path，用逗号分隔
                for path in value.split(',').map(str::trim) {
                    if !path.is_empty() {
                        if let Err(e) = paths.insert(path.to_string(), true) {
                            warn!("Failed to insert path '{}': {}", path, e);
                        }
                    }
                }

                RoutePredicateFactory::PathPredicates(PathRoutePredicateFactory { paths })
            }

            // =============================
            // Query 谓词
            // =============================
            "Query" => {
                let name = value.trim().to_string();
                if name.is_empty() {
                    warn!("Query predicate requires a parameter name");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::QueryPredicates(QueryRoutePredicateFactory { name })
            }

            // =============================
            // RemoteAddr 谓词
            // =============================
            "RemoteAddr" => {
                let remote_addr = value.trim().to_string();
                if remote_addr.is_empty() {
                    warn!("RemoteAddr predicate requires an address");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::RemoteAddrPredicates(RemoteAddrRoutePredicateFactory {
                    remote_addr,
                })
            }

            // =============================
            // XForwardedRemoteAddr 谓词
            // =============================
            "XForwardedRemoteAddr" => {
                let ips = value.trim_end().to_string();
                if ips.is_empty() {
                    warn!("XForwardedRemoteAddr predicate requires an address");
                    return RoutePredicateFactory::Nothing;
                }

                let allowed_ips = ips.split(",").map(|s| s.to_string()).collect::<Vec<_>>();
                if allowed_ips.is_empty() {
                    warn!("XForwardedRemoteAddr predicate requires at least one address");
                    return RoutePredicateFactory::Nothing;
                }

                RoutePredicateFactory::XForwardedRemoteAddr(
                    XForwardedRemoteAddrRoutePredicateFactory { allowed_ips },
                )
            }

            // =============================
            // 未知谓词
            // =============================
            _ => {
                warn!("Unsupported predicate: {}", key);
                RoutePredicateFactory::Nothing
            }
        }
    }
}
