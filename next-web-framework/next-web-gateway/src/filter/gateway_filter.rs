use pingora::{
    http::{RequestHeader, ResponseHeader},
    proxy::Session,
};

use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

use super::{
    add_request_header::AddRequestHeaderFilter,
    add_request_headers_if_not_present::AddRequestHeaderIfNotPresentFilter,
    add_response_header::AddResponseHeaderFilter, remove_request_header::RemoveRequestHeaderFilter,
    request_size::RequestSizeFilter, rewrite_path::RewritePathFilter,
    rewrite_response_header::RewriteResponseHeaderFilter, save_session::SaveSessionFilter,
    set_request_header::SetRequestHeaderFilter,
    set_request_host_header::SetRequestHostHeaderFilter,
    set_response_header::SetResponseHeaderFilter, set_status::SetStatusFilter,
    strip_prefix::StripPrefixFilter,
};

pub trait DefaultGatewayFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut RequestHeader,
        respnose_header: &mut ResponseHeader,
    );
}

#[derive(Debug, Clone)]
pub enum GatewayFilter {
    AddRequestHeader(AddRequestHeaderFilter),
    AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter),
    AddResponseHeader(AddResponseHeaderFilter),
    RemoveRequestHeader(RemoveRequestHeaderFilter),
    RequestSize(RequestSizeFilter),
    RewritePath(RewritePathFilter),
    RewriteResponseHeader(RewriteResponseHeaderFilter),
    SaveSession(SaveSessionFilter),
    SetRequestHeader(SetRequestHeaderFilter),
    SetRequestHostHeader(SetRequestHostHeaderFilter),
    SetResponseHeader(SetResponseHeaderFilter),
    SetStatus(SetStatusFilter),
    StripPrefix(StripPrefixFilter),

    Nothing,
}

impl GatewayFilter {
    pub fn filter(
        &self,
        ctx: &mut ApplicationContext,
        upstream_request_header: &mut RequestHeader,
        upstream_response_header: &mut ResponseHeader,
    ) {
        match self {
            GatewayFilter::AddRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::AddRequestHeaderIfNotPresent(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::AddResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::RemoveRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::RequestSize(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::RewritePath(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::RewriteResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::SaveSession(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::SetRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::SetRequestHostHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::SetResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::SetStatus(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            GatewayFilter::StripPrefix(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            // GatewayFilter::CircuitBreaker(gateway_filter) => {
            //     gateway_filter.filter(session, upstream_request_header, upstream_response_header)
            // }
            GatewayFilter::Nothing => {}
        }
    }
}

impl Into<GatewayFilter> for &String {
    fn into(self) -> GatewayFilter {
        if self.is_empty() {
            panic!("empty filter name")
        }
        let (filter_name, value) = if !self.contains("=") {
            (self.as_str(), "")
        } else {
            self.split_once('=').unwrap()
        };

        match filter_name.trim_end() {
            "AddRequestHeader" => {
                let headers = handle_one(value);
                GatewayFilter::AddRequestHeader(AddRequestHeaderFilter { headers })
            }
            "AddRequestHeaderIfNotPresent" => {
                let headers = handle_one(value);
                GatewayFilter::AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter {
                    headers,
                })
            }
            "AddResponseHeader" => {
                let headers = handle_one(value);
                GatewayFilter::AddResponseHeader(AddResponseHeaderFilter { headers })
            }
            "RemoveRequestHeader" => {
                let headers = handle_two(value);
                GatewayFilter::RemoveRequestHeader(RemoveRequestHeaderFilter { headers })
            }
            // default value is 10MB
            "RequestSize" => GatewayFilter::RequestSize(RequestSizeFilter {
                max_size: value.parse().unwrap_or(10485760),
            }),

            // TODO
            "RewritePath" => GatewayFilter::RewritePath(RewritePathFilter {}),
            // TODO
            "RewriteResponseHeader" => {
                GatewayFilter::RewriteResponseHeader(RewriteResponseHeaderFilter {})
            }
            "SaveSession" => GatewayFilter::SaveSession(SaveSessionFilter {}),
            "SetRequestHeader" => {
                let headers = handle_one(value);
                GatewayFilter::SetRequestHeader(SetRequestHeaderFilter { headers })
            }

            "SetRequestHostHeader" => {
                let host = value.to_string();
                GatewayFilter::SetRequestHostHeader(SetRequestHostHeaderFilter { host })
            }
            "SetResponseHeader" => {
                let headers = handle_one(value);
                GatewayFilter::SetResponseHeader(SetResponseHeaderFilter { headers })
            }

            "SetStatus" => GatewayFilter::SetStatus(SetStatusFilter {
                status: value.parse().unwrap_or(200),
            }),
            "StripPrefix" => GatewayFilter::StripPrefix(StripPrefixFilter {
                offset: value.parse().unwrap_or(0),
            }),
            // "CircuitBreaker" => {
            //     let id = value.to_string();
            //     GatewayFilter::CircuitBreaker(CircuitBreakerFilter { id })
            // }
            _ => GatewayFilter::Nothing,
        }
    }
}

// Split -> X-Request-red:blue,X-Request-green:yellow
fn handle_one(str: &str) -> Vec<KeyValue<String>> {
    str.trim_end()
        .split(',')
        .collect::<Vec<&str>>()
        .iter()
        .map(|s| s.trim_end().split(":").collect::<Vec<&str>>())
        .map(|s| Into::<KeyValue<String>>::into(s))
        .collect()
}

// Split -> X-Request-Foo,X-Request-Bar
fn handle_two(str: &str) -> Vec<String> {
    str.trim_end()
        .split(',')
        .map(|s| s.trim_end().to_string())
        .collect()
}
