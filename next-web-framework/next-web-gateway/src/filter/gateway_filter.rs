use pingora::http::{RequestHeader, ResponseHeader};
use regex::Regex;

use crate::application::next_gateway_application::ApplicationContext;
use crate::filter::secure_headers::SecureHeadersFilter;
use crate::filter::set_path::SetPathFilter;
use crate::{filter::add_request_parameter::AddRequestParameterFilter, util::key_value::KeyValue};

use super::map_request_header::MapRequestHeaderFilter;
use super::prefix_path::PrefixPathFilter;
use super::preserve_host_header::PreserveHostHeaderFilter;
use super::redirect_to::RedirectToFilter;
use super::remove_response_header::RemoveResponseHeaderFilter;
use super::request_header_size::RequestHeaderSizeFilter;
use super::request_rate_limiter::RequestRateLimiterFilter;
use super::rewrite_location_response_header::RewriteLocationResponseHeaderFilter;
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

macro_rules! delegate_filter {
    ($self:ident, $ctx:ident, $req:ident, $resp:ident, $($variant:ident),*) => {
        match $self {
            $(Self::$variant(filter) => filter.filter($ctx, $req, $resp),)*
            Self::Nothing => {}
        }
    };
}

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
        delegate_filter!(
            self,
            ctx,
            upstream_request_header,
            upstream_response_header,
            AddRequestHeader,
            AddRequestHeaderIfNotPresent,
            AddResponseHeader,
            AddRequestParameter,
            MapRequestHeader,
            PrefixPath,
            PreserveHostHeader,
            RedirectTo,
            RemoveRequestHeader,
            RemoveResponseHeader,
            RequestHeaderSize,
            RequestRateLimiter,
            RequestSize,
            RewriteLocationResponseHeader,
            RewritePath,
            RewriteResponseHeader,
            SaveSession,
            SecureHeaders,
            SetRequestHeader,
            SetPath,
            SetRequestHostHeader,
            SetResponseHeader,
            SetStatus,
            StripPrefix
        );
    }
}

impl From<&str> for DefaultGatewayFilter {

    fn from(str: &str) -> Self {
        if str.is_empty() {
            panic!("empty filter name");
        }

        let (filter_name, value) = if let Some(pos) = str.find('=') {
            (&str[..pos], &str[pos + 1..])
        } else {
            (str, "")
        };

        match filter_name.trim() {
            "AddRequestHeader" => Self::AddRequestHeader(AddRequestHeaderFilter {
                headers: split_kv_pairs(value),
            }),

            "AddRequestHeaderIfNotPresent" => Self::AddRequestHeaderIfNotPresent(
                AddRequestHeaderIfNotPresentFilter {
                    headers: split_kv_pairs(value),
                },
            ),

            "AddRequestParameter" => {
                let parameters = split_params(value, "&", ",");
                if parameters.is_empty() {
                    Self::Nothing
                } else {
                    Self::AddRequestParameter(AddRequestParameterFilter { parameters })
                }
            }

            "AddResponseHeader" => Self::AddResponseHeader(AddResponseHeaderFilter {
                headers: split_kv_pairs(value),
            }),

            "MapRequestHeader" => {
                if let Some((key, value)) = value.split_once(',') {
                    Self::MapRequestHeader(MapRequestHeaderFilter {
                        header: KeyValue::from((key, value)),
                    })
                } else {
                    Self::Nothing
                }
            }

            "PrefixPath" => Self::PrefixPath(PrefixPathFilter {
                path: value.trim().into(),
            }),

            "PreserveHostHeader" => Self::PreserveHostHeader(PreserveHostHeaderFilter {}),

            "RedirectTo" => {
                if let Some((status_str, url)) = value.split_once(',') {
                    if let Ok(status) = status_str.trim().parse() {
                        Self::RedirectTo(RedirectToFilter {
                            status,
                            url: url.trim().into(),
                        })
                    } else {
                        Self::Nothing
                    }
                } else {
                    Self::Nothing
                }
            }

            "RemoveRequestHeader" => Self::RemoveRequestHeader(RemoveRequestHeaderFilter {
                headers: split_headers(value),
            }),

            "RemoveResponseHeader" => Self::RemoveResponseHeader(RemoveResponseHeaderFilter {
                headers: split_headers(value),
            }),

            "RequestHeaderSize" => {
                if let Some((max_size_str, error_msg)) = value.split_once(',') {
                    Self::RequestHeaderSize(RequestHeaderSizeFilter {
                        max_size: max_size_str.trim().parse().unwrap_or(0),
                        error_message: error_msg.trim().to_string(),
                    })
                } else {
                    Self::RequestHeaderSize(RequestHeaderSizeFilter {
                        max_size: value.trim().parse().unwrap_or(0),
                        error_message: "Request header size exceeded".to_string(),
                    })
                }
            }

            "RequestRateLimiter" => Self::RequestRateLimiter(RequestRateLimiterFilter {
                rate_limit: value.trim().parse().unwrap_or(0),
            }),

            "RequestSize" => Self::RequestSize(RequestSizeFilter {
                max_size: value.trim().parse().unwrap_or(0),
            }),

            "RewriteLocationResponseHeader" => {
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                Self::RewriteLocationResponseHeader(RewriteLocationResponseHeaderFilter {
                    strip_version_mode: parts.get(0).map(|&s| s.into()).unwrap_or_default(),
                    location_header_name: parts.get(1).map(|&s| Some(s.into())).unwrap_or_default(),
                    host_value: parts.get(2).map(|&s| Some(s.into())).unwrap_or_default(),
                    protocols_regex: parts.get(3).and_then(|&s| Regex::new(s).ok()),
                })
            }

            "RewriteResponseHeader" => {
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let regex = parts.get(2).and_then(|&s| Regex::new(s).ok());
                    Self::RewriteResponseHeader(RewriteResponseHeaderFilter {
                        header: (
                            KeyValue::from((parts[0], parts[1])),
                            regex,
                        ),
                    })
                } else {
                    Self::Nothing
                }
            }

            "SaveSession" => Self::SaveSession(SaveSessionFilter {}),
            "SecureHeaders" => Self::SecureHeaders(SecureHeadersFilter {}),

            "SetRequestHeader" => Self::SetRequestHeader(SetRequestHeaderFilter {
                headers: split_kv_pairs(value),
            }),

            "SetPath" => Self::SetPath(SetPathFilter {
                path: value.trim().to_string(),
            }),

            "SetRequestHostHeader" => Self::SetRequestHostHeader(SetRequestHostHeaderFilter {
                host: value.trim().to_string(),
            }),

            "SetResponseHeader" => Self::SetResponseHeader(SetResponseHeaderFilter {
                headers: split_kv_pairs(value),
            }),

            "SetStatus" => Self::SetStatus(SetStatusFilter {
                status: value.trim().parse().unwrap_or(200),
            }),

            "StripPrefix" => Self::StripPrefix(StripPrefixFilter {
                offset: value.trim().parse().unwrap_or(0),
            }),

            _ => Self::Nothing,
        }
    }
}

fn split(str: &str) -> Vec<KeyValue<String, String>> {
    str.trim()
        .split(',')
        .map(|s| s.trim().split(":").collect::<Vec<&str>>())
        .map(|s| Into::<KeyValue<String, String>>::into(s))
        .collect()
}

// 使用更高效的字符串分割和处理方式
fn split_kv_pairs(str: &str) -> Vec<KeyValue<String, String>> {
    str.trim()
        .split(',')
        .filter_map(|pair| {
            let mut parts = pair.trim().splitn(2, ':');
            let key = parts.next()?.trim();
            let value = parts.next()?.trim();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some(KeyValue::from((key, value)))
            }
        })
        .collect()
}

fn split_headers(str: &str) -> Vec<String> {
    str.trim()
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

fn split_params<'a>(
    input: &'a str,
    param_delimiter: &'a str,
    kv_delimiter: &'a str,
) -> Vec<KeyValue<String, String>> {
    input
        .trim()
        .split(param_delimiter)
        .filter_map(|param| {
            let mut parts = param.trim().splitn(2, kv_delimiter);
            let key = parts.next()?.trim();
            let value = parts.next()?.trim();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some(KeyValue::from((key, value)))
            }
        })
        .collect()
}
