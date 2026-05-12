use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pingora_limits::rate::Rate;
use regex::Regex;

use crate::error::GatewayError;
use crate::filter::factory::add_request_header::AddRequestHeaderFilter;
use crate::filter::factory::add_request_headers_if_not_present::AddRequestHeaderIfNotPresentFilter;
use crate::filter::factory::add_request_parameter::AddRequestParameterFilter;
use crate::filter::factory::add_response_header::AddResponseHeaderFilter;
use crate::filter::factory::dedupe_response_header::DedupeResponseHeaderFilter;
use crate::filter::factory::local_response_cache::LocalResponseCacheFilter;
use crate::filter::factory::map_request_header::MapRequestHeaderFilter;
use crate::filter::factory::prefix_path::PrefixPathFilter;
use crate::filter::factory::preserve_host_header::PreserveHostHeaderFilter;
use crate::filter::factory::redirect_to::RedirectToFilter;
use crate::filter::factory::remove_json_attributes_response_body::RemoveJsonAttributesResponseBodyFilter;
use crate::filter::factory::remove_request_header::RemoveRequestHeaderFilter;
use crate::filter::factory::remove_request_parameter::RemoveRequestParameterFilter;
use crate::filter::factory::remove_response_header::RemoveResponseHeaderFilter;
use crate::filter::factory::request_header_size::RequestHeaderSizeFilter;
use crate::filter::factory::request_rate_limiter::RequestRateLimiterFilter;
use crate::filter::factory::request_size::RequestSizeFilter;
use crate::filter::factory::rewrite_location_response_header::RewriteLocationResponseHeaderFilter;
use crate::filter::factory::rewrite_path::RewritePathFilter;
use crate::filter::factory::rewrite_response_header::RewriteResponseHeaderFilter;
use crate::filter::factory::save_session::SaveSessionFilter;
use crate::filter::factory::secure_headers::SecureHeadersFilter;
use crate::filter::factory::set_path::SetPathFilter;
use crate::filter::factory::set_request_header::SetRequestHeaderFilter;
use crate::filter::factory::set_request_host_header::SetRequestHostHeaderFilter;
use crate::filter::factory::set_response_header::SetResponseHeaderFilter;
use crate::filter::factory::set_status::SetStatusFilter;
use crate::filter::factory::strip_prefix::StripPrefixFilter;
use crate::filter::factory::token_relay::TokenRelayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

macro_rules! delegate_filter {
    ($self:ident, $exchange:ident, $chain:ident, $($variant:ident),*) => {
        match $self {
            $(Self::$variant(filter) => filter.filter($exchange, $chain).await,)*
            Self::Nothing => $chain.filter($exchange).await,
        }
    };
}

/// Name key.
pub const NAME_KEY: &str = "name";

/// Value key.
pub const VALUE_KEY: &str = "value";

#[async_trait]
pub trait GatewayFilter
where
    Self: Send + Sync,
{
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError>;
}

#[derive(Debug, Clone)]
pub enum DefaultGatewayFilter {
    AddRequestHeader(AddRequestHeaderFilter),
    AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter),
    AddRequestParameter(AddRequestParameterFilter),
    AddResponseHeader(AddResponseHeaderFilter),
    DedupeResponseHeader(DedupeResponseHeaderFilter),
    MapRequestHeader(MapRequestHeaderFilter),
    PrefixPath(PrefixPathFilter),
    PreserveHostHeader(PreserveHostHeaderFilter),
    RedirectTo(RedirectToFilter),
    LocalResponseCache(LocalResponseCacheFilter),
    RemoveJsonAttributesResponseBody(RemoveJsonAttributesResponseBodyFilter),
    RemoveRequestHeader(RemoveRequestHeaderFilter),
    RemoveRequestParameter(RemoveRequestParameterFilter),
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
    TokenRelay(TokenRelayFilter),

    Nothing,
}

#[async_trait]
impl GatewayFilter for DefaultGatewayFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        delegate_filter!(
            self,
            exchange,
            chain,
            AddRequestHeader,
            AddRequestHeaderIfNotPresent,
            AddResponseHeader,
            AddRequestParameter,
            DedupeResponseHeader,
            MapRequestHeader,
            PrefixPath,
            PreserveHostHeader,
            RedirectTo,
            LocalResponseCache,
            RemoveJsonAttributesResponseBody,
            RemoveRequestHeader,
            RemoveRequestParameter,
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
            StripPrefix,
            TokenRelay
        )
    }
}

impl DefaultGatewayFilter {
    pub fn modifies_response_body(&self) -> bool {
        matches!(self, Self::RemoveJsonAttributesResponseBody(_))
    }
}

impl From<&str> for DefaultGatewayFilter {
    fn from(str: &str) -> Self {
        if str.is_empty() {
            return Self::Nothing;
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

            "AddRequestHeaderIfNotPresent" => {
                Self::AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter {
                    headers: split_kv_pairs(value),
                })
            }

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

            "DedupeResponseHeader" => {
                let filter = DedupeResponseHeaderFilter::from(value);
                if filter.headers.is_empty() {
                    Self::Nothing
                } else {
                    Self::DedupeResponseHeader(filter)
                }
            }

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

            "LocalResponseCache" => {
                let filter = LocalResponseCacheFilter::from(value);
                if filter.is_valid() {
                    Self::LocalResponseCache(filter)
                } else {
                    Self::Nothing
                }
            }

            "RemoveJsonAttributesResponseBody" => {
                let filter = RemoveJsonAttributesResponseBodyFilter::from(value);
                if filter.names.is_empty() {
                    Self::Nothing
                } else {
                    Self::RemoveJsonAttributesResponseBody(filter)
                }
            }

            "RemoveRequestHeader" => Self::RemoveRequestHeader(RemoveRequestHeaderFilter {
                headers: split_headers(value),
            }),

            "RemoveRequestParameter" => {
                Self::RemoveRequestParameter(RemoveRequestParameterFilter {
                    names: split_headers(value),
                })
            }

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
                limiter: Arc::new(Rate::new(Duration::from_secs(1))),
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
                        header: (KeyValue::from((parts[0], parts[1])), regex),
                    })
                } else {
                    Self::Nothing
                }
            }

            "RewritePath" => {
                if let Some((regex, replacement)) = value.split_once(',') {
                    if let Ok(regex) = Regex::new(regex.trim()) {
                        Self::RewritePath(RewritePathFilter {
                            regex,
                            replacement: replacement.trim().to_string(),
                        })
                    } else {
                        Self::Nothing
                    }
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

            "TokenRelay" => Self::TokenRelay(TokenRelayFilter {}),

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
