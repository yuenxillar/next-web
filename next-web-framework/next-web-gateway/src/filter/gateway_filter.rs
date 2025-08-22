use pingora::http::{RequestHeader, ResponseHeader};

use crate::application::next_gateway_application::ApplicationContext;
use crate::filter::secure_headers::SecureHeadersFilter;
use crate::filter::set_path::SetPathFilter;
use crate::{filter::add_request_parameter::AddRequestParameterFilter, util::key_value::KeyValue};

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

pub trait GatewayFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut RequestHeader,
        respnose_header: &mut ResponseHeader,
    );
}

#[derive(Debug, Clone)]
pub enum DefaultGatewayFilter {
    AddRequestHeader(AddRequestHeaderFilter),
    AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter),
    AddRequestParameter(AddRequestParameterFilter),
    AddResponseHeader(AddResponseHeaderFilter),
    MapRequestHeader(MapRequestHeaderFilter),
    PrefixPath(PrefixPathFilter),
    PreserveHostHeader(PreserveHostHeaderFilter),
    RedirectTo(RedirectToFilter),
    RemoveRequestHeader(RemoveRequestHeaderFilter),
    RemoveResponseHeader(RemoveResponseHeaderFilter),
    RequestHeaderSize(RequestHeaderSizeFilter),
    RequestRateLimiter(RequestRateLimiterFilter),
    RequestSize(RequestSizeFilter),
    RewriteLocationResponseHeader(RewriteLocationResponseHeaderFilter),
    RewritePath(RewritePathFilter),
    RewriteResponseHeader(RewriteResponseHeaderFilter),
    SaveSession(SaveSessionFilter),
    SecureHeaders(SecureHeadersFilter),
    SetRequestHeader(SetRequestHeaderFilter),
    SetPath(SetPathFilter),
    SetRequestHostHeader(SetRequestHostHeaderFilter),
    SetResponseHeader(SetResponseHeaderFilter),
    SetStatus(SetStatusFilter),
    StripPrefix(StripPrefixFilter),

    Nothing,
}

impl DefaultGatewayFilter {
    pub fn filter(
        &self,
        ctx: &mut ApplicationContext,
        upstream_request_header: &mut RequestHeader,
        upstream_response_header: &mut ResponseHeader,
    ) {
        match self {
            Self::AddRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::AddRequestHeaderIfNotPresent(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::AddResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::RemoveRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::RequestSize(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::RewritePath(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::RewriteResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SaveSession(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SecureHeaders(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SetRequestHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SetPath(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SetRequestHostHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SetResponseHeader(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::SetStatus(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::StripPrefix(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::AddRequestParameter(gateway_filter) => {
                gateway_filter.filter(ctx, upstream_request_header, upstream_response_header)
            }
            Self::Nothing => {}
        }
    }
}

impl Into<DefaultGatewayFilter> for &String {
    fn into(self) -> DefaultGatewayFilter {
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
                let headers = split_one(value);
                DefaultGatewayFilter::AddRequestHeader(AddRequestHeaderFilter { headers })
            }
            "AddRequestHeaderIfNotPresent" => {
                let headers = split_one(value);
                DefaultGatewayFilter::AddRequestHeaderIfNotPresent(
                    AddRequestHeaderIfNotPresentFilter { headers },
                )
            }
            "AddRequestParameter" => {
                DefaultGatewayFilter::AddRequestParameter(AddRequestParameterFilter {
                    parameter: todo!(),
                })
            }

            "AddResponseHeader" => {
                let headers = split_one(value);
                DefaultGatewayFilter::AddResponseHeader(AddResponseHeaderFilter { headers })
            }

            "MapRequestHeader" => DefaultGatewayFilter::MapRequestHeader(),
            // TODO
            "PrefixPath" => DefaultGatewayFilter::PrefixPath(),

            // TODO
            "PreserveHostHeader" => DefaultGatewayFilter::PreserveHostHeader(),

            // TODO
            "RedirectTo" => DefaultGatewayFilter::RedirectTo(),

            "RemoveRequestHeader" => {
                let headers = split_two(value);
                DefaultGatewayFilter::RemoveRequestHeader(RemoveRequestHeaderFilter { headers })
            }

            // TODO
            "RemoveResponseHeader" => DefaultGatewayFilter::RemoveResponseHeader(),
            // TODO
            "RequestHeaderSize" => DefaultGatewayFilter::RequestHeaderSize(),

            // TODO
            "RequestRateLimiter" => DefaultGatewayFilter::RequestRateLimiter(),

            // default value is 10MB
            "RequestSize" => DefaultGatewayFilter::RequestSize(RequestSizeFilter {
                max_size: value.parse().unwrap_or(10485760),
            }),

            // TODO
            "RewriteLocationResponseHeader" => {
                DefaultGatewayFilter::RewriteLocationResponseHeader()
            }

            // TODO
            "RewritePath" => DefaultGatewayFilter::RewritePath(RewritePathFilter {}),

            "RewriteResponseHeader" => {
                let header = split_two(value);
                let regex = if header.len() == 2 {
                    None
                } else {
                    Some(regex::Regex::new(&header[2]).unwrap())
                };
                DefaultGatewayFilter::RewriteResponseHeader(RewriteResponseHeaderFilter {
                    header: (
                        KeyValue {
                            k: header[0].to_string(),
                            v: header[1].to_string(),
                        },
                        regex,
                    ),
                })
            }
            "SaveSession" => DefaultGatewayFilter::SaveSession(SaveSessionFilter {}),

            // TODO
            "SecureHeaders" => DefaultGatewayFilter::SecureHeaders(SecureHeadersFilter {}),

            "SetRequestHeader" => {
                let headers = split_one(value);
                DefaultGatewayFilter::SetRequestHeader(SetRequestHeaderFilter { headers })
            }

            // TODO
            "SetPath" => DefaultGatewayFilter::SetPath(),

            "SetRequestHostHeader" => {
                let host = value.to_string();
                DefaultGatewayFilter::SetRequestHostHeader(SetRequestHostHeaderFilter { host })
            }
            "SetResponseHeader" => {
                let headers = split_one(value);
                DefaultGatewayFilter::SetResponseHeader(SetResponseHeaderFilter { headers })
            }

            "SetStatus" => DefaultGatewayFilter::SetStatus(SetStatusFilter {
                status: value.parse().unwrap_or(200),
            }),
            "StripPrefix" => DefaultGatewayFilter::StripPrefix(StripPrefixFilter {
                offset: value.parse().unwrap_or(0),
            }),

            _ => DefaultGatewayFilter::Nothing,
        }
    }
}

// Split -> X-Request-red:blue,X-Request-green:yellow
fn split_one(str: &str) -> Vec<KeyValue<String>> {
    str.trim_end()
        .split(',')
        .map(|s| s.trim_end().split(":").collect::<Vec<&str>>())
        .map(|s| Into::<KeyValue<String>>::into(s))
        .collect()
}

// Split -> X-Request-Foo,X-Request-Bar
fn split_two(str: &str) -> Vec<String> {
    str.trim_end()
        .split(',')
        .map(|s| s.trim_end().to_string())
        .collect()
}
