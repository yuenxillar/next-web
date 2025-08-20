use core::panic;

use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono_tz::Tz;
use matchit::Router;
use pingora::http::Method;
use pingora::proxy::Session;

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
    Nothing,
}

impl RoutePredicateFactory {
    pub fn matches(&self, session: &mut Session) -> bool {
        let result = match self {
            RoutePredicateFactory::ZonedDateTimePredicates(factory) => factory.matches(session),
            RoutePredicateFactory::CookiePredicates(factory) => factory.matches(session),
            RoutePredicateFactory::HeaderPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::HostPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::MethodPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::PathPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::QueryPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::RemoteAddrPredicates(factory) => factory.matches(session),
            RoutePredicateFactory::Nothing => false,
        };
        result
    }
}

impl Into<RoutePredicateFactory> for &String {
    fn into(self) -> RoutePredicateFactory {
        if let Some((str1, str2)) = self.split_once('=') {
            if str1.is_empty() {
                return RoutePredicateFactory::Nothing;
            }
            if str2.is_empty() {
                panic!("Invalid remote address format");
            }

            return match str1 {
                "Before" | "After" | "Between" => {
                    // 先尝试提取出时间部分和时区部分
                    let (datetime, offset) = {
                        let mut datetime = Vec::new();
                        let var = {
                            if str1 == "Between" {
                                str2.trim_end().split(",").collect::<Vec<&str>>()
                            } else {
                                vec![str2]
                            }
                        };
                        for s in var {
                            if s.is_empty() {
                                continue;
                            }
                            let parts: Vec<&str> = str2.splitn(2, '[').collect();
                            if parts.len() != 2 {
                                panic!("Invalid time format");
                            }
                            let time_part = parts[0];
                            let tz_part = parts[1].trim_end_matches(']');
                            // 解析时区
                            let timezone: Tz = match tz_part.parse() {
                                Ok(tz) => tz,
                                Err(_) => {
                                    panic!("Failed to parse timezone");
                                }
                            };
                            // 解析日期时间部分
                            let naive_datetime = match NaiveDateTime::parse_from_str(
                                time_part,
                                "%Y-%m-%dT%H:%M:%S%.3f%z",
                            ) {
                                Ok(ndt) => ndt,
                                Err(_) => {
                                    panic!("Failed to parse datetime");
                                }
                            };
                            // 结合时区和日期时间
                            datetime.push(timezone.from_local_datetime(&naive_datetime).unwrap());
                        }
                        let offset = if str1 == "Before" {
                            0
                        } else if str1 == "After" {
                            1
                        } else {
                            2
                        };
                        (datetime, offset)
                    };
                    RoutePredicateFactory::ZonedDateTimePredicates(
                        ZonedDateTimeRoutePredicateFactory { datetime, offset },
                    )
                }
                "Cookie" => {
                    let cookies: Vec<String> = str2
                        .trim_end()
                        .split(",")
                        .collect::<Vec<&str>>()
                        .iter()
                        .map(|f| f.to_string())
                        .collect();

                    RoutePredicateFactory::CookiePredicates(CookieRoutePredicateFactory { cookies })
                }
                "Header" => {
                    let header = StrUtil::parse_kv_one_and_option(str2);
                    let regex = header
                        .v
                        .clone()
                        .map(|f| Some(regex::Regex::new(&f).unwrap()))
                        .unwrap_or_default();
                    RoutePredicateFactory::HeaderPredicates(HeaderRoutePredicateFactory {
                        header,
                        regex,
                    })
                }
                "Host" => {
                    let hosts = str2
                        .trim_end()
                        .split(",")
                        .collect::<Vec<&str>>()
                        .iter()
                        .map(|f| f.to_string())
                        .collect::<Vec<String>>();
                    RoutePredicateFactory::HostPredicates(HostRoutePredicateFactory { hosts })
                }
                "Method" => {
                    let method = str2
                        .trim_end()
                        .split(',')
                        .collect::<Vec<&str>>()
                        .iter()
                        .map(|f| f.to_uppercase())
                        .collect::<Vec<String>>();
                    let methods = method
                        .iter()
                        .map(|f| Method::from_bytes(f.as_bytes()).unwrap())
                        .collect::<Vec<Method>>();
                    RoutePredicateFactory::MethodPredicates(MethodRoutePredicateFactory { methods })
                }
                "Path" => {
                    let mut paths = Router::new();

                    // let _ = str2
                    //     .trim_end()
                    //     .split(",")
                    //     .collect::<Vec<&str>>()
                    //     .iter()
                    //     .map(|f| {
                    //         println!("f: {}", f);
                    //         paths.insert(f.to_string(), true).unwrap()
                    // });
                    paths.insert(str2.trim_end().to_string(), true).unwrap();
                    RoutePredicateFactory::PathPredicates(PathRoutePredicateFactory { paths })
                }
                "Query" => {
                    let name = str2.trim_end().to_string();
                    RoutePredicateFactory::QueryPredicates(QueryRoutePredicateFactory { name })
                }
                "RemoteAddr" => {
                    let remote_addr = str2.trim_end().to_string();
                    RoutePredicateFactory::RemoteAddrPredicates(RemoteAddrRoutePredicateFactory {
                        remote_addr,
                    })
                }
                _ => RoutePredicateFactory::Nothing,
            };
        }
        RoutePredicateFactory::Nothing
    }
}
