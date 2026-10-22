//! The filter catalogue and its string DSL parser.
//!
//! Every filter belongs to exactly one [`FilterPhase`], and the parser rejects
//! unknown names and malformed arguments at startup instead of silently
//! disabling the filter.

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pingora_limits::rate::Rate;
use regex::Regex;

use crate::config::ConfigError;
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
use crate::filter::factory::secure_headers::SecureHeadersFilter;
use crate::filter::factory::set_path::SetPathFilter;
use crate::filter::factory::set_request_header::SetRequestHeaderFilter;
use crate::filter::factory::set_request_host_header::SetRequestHostHeaderFilter;
use crate::filter::factory::set_response_header::SetResponseHeaderFilter;
use crate::filter::factory::set_status::SetStatusFilter;
use crate::filter::factory::strip_prefix::StripPrefixFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::filter::spec::{expect_args, unknown_name_error, FilterPhase, FilterSpec};
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

macro_rules! delegate_filter {
    ($self:ident, $exchange:ident, $chain:ident, $($variant:ident),*) => {
        match $self {
            $(Self::$variant(filter) => filter.filter($exchange, $chain).await,)*
        }
    };
}

/// Every filter name the parser accepts.
pub const KNOWN_FILTERS: &[&str] = &[
    "AddRequestHeader",
    "AddRequestHeadersIfNotPresent",
    "AddRequestParameter",
    "AddResponseHeader",
    "DedupeResponseHeader",
    "LocalResponseCache",
    "MapRequestHeader",
    "PrefixPath",
    "PreserveHostHeader",
    "RedirectTo",
    "RemoveJsonAttributesResponseBody",
    "RemoveRequestHeader",
    "RemoveRequestParameter",
    "RemoveResponseHeader",
    "RequestHeaderSize",
    "RequestRateLimiter",
    "RequestSize",
    "RewriteLocationResponseHeader",
    "RewritePath",
    "RewriteResponseHeader",
    "SecureHeaders",
    "SetPath",
    "SetRequestHeader",
    "SetRequestHostHeader",
    "SetResponseHeader",
    "SetStatus",
    "StripPrefix",
];

/// A gateway filter, one variant per configuration name.
#[async_trait]
pub trait GatewayFilter
where
    Self: Send + Sync,
{
    /// Run the filter; call `chain.filter(exchange)` to continue.
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError>;
}

/// The built-in filter catalogue.
#[derive(Debug, Clone)]
pub enum DefaultGatewayFilter {
    /// Append request headers.
    AddRequestHeader(AddRequestHeaderFilter),
    /// Insert request headers that are not already present.
    AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter),
    /// Append query parameters to the upstream request.
    AddRequestParameter(AddRequestParameterFilter),
    /// Append response headers.
    AddResponseHeader(AddResponseHeaderFilter),
    /// Collapse duplicated response header values.
    DedupeResponseHeader(DedupeResponseHeaderFilter),
    /// Serve and store responses from the in-process cache.
    LocalResponseCache(LocalResponseCacheFilter),
    /// Copy one request header onto another.
    MapRequestHeader(MapRequestHeaderFilter),
    /// Prefix the upstream request path.
    PrefixPath(PrefixPathFilter),
    /// Keep the downstream `Host` header.
    PreserveHostHeader(PreserveHostHeaderFilter),
    /// Answer with a redirect instead of proxying.
    RedirectTo(RedirectToFilter),
    /// Strip attributes from a JSON response body.
    RemoveJsonAttributesResponseBody(RemoveJsonAttributesResponseBodyFilter),
    /// Remove request headers.
    RemoveRequestHeader(RemoveRequestHeaderFilter),
    /// Remove query parameters from the upstream request.
    RemoveRequestParameter(RemoveRequestParameterFilter),
    /// Remove response headers.
    RemoveResponseHeader(RemoveResponseHeaderFilter),
    /// Reject requests whose header block is too large.
    RequestHeaderSize(RequestHeaderSizeFilter),
    /// Limit request admission rate.
    RequestRateLimiter(RequestRateLimiterFilter),
    /// Reject request bodies larger than a limit.
    RequestSize(RequestSizeFilter),
    /// Rewrite the `Location` response header.
    RewriteLocationResponseHeader(RewriteLocationResponseHeaderFilter),
    /// Rewrite the upstream request path with a regular expression.
    RewritePath(RewritePathFilter),
    /// Rewrite a response header value with a regular expression.
    RewriteResponseHeader(RewriteResponseHeaderFilter),
    /// Add security hardening response headers.
    SecureHeaders(SecureHeadersFilter),
    /// Replace the upstream request path.
    SetPath(SetPathFilter),
    /// Replace request headers.
    SetRequestHeader(SetRequestHeaderFilter),
    /// Replace the upstream `Host` header.
    SetRequestHostHeader(SetRequestHostHeaderFilter),
    /// Replace response headers.
    SetResponseHeader(SetResponseHeaderFilter),
    /// Replace the response status code.
    SetStatus(SetStatusFilter),
    /// Drop leading path segments.
    StripPrefix(StripPrefixFilter),
}

impl DefaultGatewayFilter {
    /// The proxy pass this filter runs in.
    pub fn phase(&self) -> FilterPhase {
        match self {
            Self::RequestSize(_) => FilterPhase::RequestBody,
            Self::RemoveJsonAttributesResponseBody(_) => FilterPhase::ResponseBody,
            Self::AddResponseHeader(_)
            | Self::DedupeResponseHeader(_)
            | Self::LocalResponseCache(_)
            | Self::RemoveResponseHeader(_)
            | Self::RewriteLocationResponseHeader(_)
            | Self::RewriteResponseHeader(_)
            | Self::SecureHeaders(_)
            | Self::SetResponseHeader(_)
            | Self::SetStatus(_) => FilterPhase::ResponseHeader,
            _ => FilterPhase::RequestHeader,
        }
    }

    /// Whether this filter rewrites the response body.
    pub fn modifies_response_body(&self) -> bool {
        self.phase() == FilterPhase::ResponseBody
    }

    /// Apply a response body filter synchronously.
    ///
    /// Body filters run on the proxy thread from `response_body_filter`, which
    /// is not async; keeping them sync removes the `block_on` that used to run
    /// a future on the worker thread.
    pub fn apply_body(&self, body: &mut bytes::Bytes) {
        if let Self::RemoveJsonAttributesResponseBody(filter) = self {
            filter.apply(body);
        }
    }

    /// Clone the filters that belong to `phase` into a chain-ready list.
    pub fn chain_for(
        filters: &[Self],
        phase: FilterPhase,
    ) -> Vec<Arc<dyn GatewayFilter>> {
        filters
            .iter()
            .filter(|filter| filter.phase() == phase)
            .cloned()
            .map(|filter| Arc::new(filter) as Arc<dyn GatewayFilter>)
            .collect()
    }

    /// Configured `name, value` header pairs, for `name`/`value` style filters.
    pub fn header_pairs(&self) -> Vec<(String, String)> {
        let pairs = |headers: &[KeyValue<String, String>]| {
            headers
                .iter()
                .map(|header| (header.k.clone(), header.v.clone()))
                .collect()
        };

        match self {
            Self::AddRequestHeader(filter) => pairs(&filter.headers),
            Self::AddRequestHeaderIfNotPresent(filter) => pairs(&filter.headers),
            Self::AddResponseHeader(filter) => pairs(&filter.headers),
            Self::SetRequestHeader(filter) => pairs(&filter.headers),
            Self::SetResponseHeader(filter) => pairs(&filter.headers),
            _ => Vec::new(),
        }
    }
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
            SecureHeaders,
            SetRequestHeader,
            SetPath,
            SetRequestHostHeader,
            SetResponseHeader,
            SetStatus,
            StripPrefix
        )
    }
}

impl TryFrom<&str> for DefaultGatewayFilter {
    type Error = ConfigError;

    /// Parse a filter expression, rejecting unknown names and bad arguments.
    fn try_from(raw: &str) -> Result<Self, ConfigError> {
        let spec = FilterSpec::parse(raw)?;

        let filter = match spec.name.as_str() {
            "AddRequestHeader" => Self::AddRequestHeader(AddRequestHeaderFilter {
                headers: parse_pairs(&spec)?,
            }),

            "AddRequestHeadersIfNotPresent" | "AddRequestHeaderIfNotPresent" => {
                if spec.name != "AddRequestHeadersIfNotPresent" {
                    tracing::warn!(
                        "filter name `AddRequestHeaderIfNotPresent` is deprecated, use \
                         `AddRequestHeadersIfNotPresent`"
                    );
                }
                Self::AddRequestHeaderIfNotPresent(AddRequestHeaderIfNotPresentFilter {
                    headers: parse_pairs(&spec)?,
                })
            }

            "AddRequestParameter" => Self::AddRequestParameter(AddRequestParameterFilter {
                parameters: parse_parameter_pairs(&spec)?,
            }),

            "AddResponseHeader" => Self::AddResponseHeader(AddResponseHeaderFilter {
                headers: parse_pairs(&spec)?,
            }),

            "DedupeResponseHeader" => {
                let filter = DedupeResponseHeaderFilter::from(spec.args.join(",").as_str());
                if filter.headers.is_empty() {
                    return Err(ConfigError::new(
                        "filters.DedupeResponseHeader",
                        "expects at least one header name",
                    ));
                }
                Self::DedupeResponseHeader(filter)
            }

            "LocalResponseCache" => {
                let filter = LocalResponseCacheFilter::from(spec.args.join(",").as_str());
                if filter.is_valid() {
                    Self::LocalResponseCache(filter)
                } else {
                    return Err(ConfigError::new(
                        "filters.LocalResponseCache",
                        "expects `ttl, max-size-bytes` with both values greater than zero",
                    ));
                }
            }

            "MapRequestHeader" => {
                let args = expect_args(&spec, 2, 2)?;
                Self::MapRequestHeader(MapRequestHeaderFilter {
                    header: KeyValue::from((args[0].as_str(), args[1].as_str())),
                })
            }

            "PrefixPath" => {
                let args = expect_args(&spec, 1, 1)?;
                Self::PrefixPath(PrefixPathFilter {
                    path: args[0].as_str().into(),
                })
            }

            "PreserveHostHeader" => {
                expect_args(&spec, 0, 0)?;
                Self::PreserveHostHeader(PreserveHostHeaderFilter {})
            }

            "RedirectTo" => {
                let args = expect_args(&spec, 2, 2)?;
                let status = parse_number::<u16>(&spec, args[0].as_str(), 300, 399)?;
                Self::RedirectTo(RedirectToFilter {
                    status,
                    url: args[1].as_str().into(),
                })
            }

            "RemoveJsonAttributesResponseBody" => {
                let filter =
                    RemoveJsonAttributesResponseBodyFilter::from(spec.args.join(",").as_str());
                if filter.names.is_empty() {
                    return Err(ConfigError::new(
                        "filters.RemoveJsonAttributesResponseBody",
                        "expects at least one attribute name",
                    ));
                }
                Self::RemoveJsonAttributesResponseBody(filter)
            }

            "RemoveRequestHeader" => Self::RemoveRequestHeader(RemoveRequestHeaderFilter {
                headers: require_names(&spec)?,
            }),

            "RemoveRequestParameter" => {
                Self::RemoveRequestParameter(RemoveRequestParameterFilter {
                    names: require_names(&spec)?,
                })
            }

            "RemoveResponseHeader" => Self::RemoveResponseHeader(RemoveResponseHeaderFilter {
                headers: require_names(&spec)?,
            }),

            "RequestHeaderSize" => {
                let args = expect_args(&spec, 1, 2)?;
                let max_size = parse_number::<u32>(&spec, args[0].as_str(), 1, u32::MAX)?;
                let error_message = args
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "Request header size exceeded".to_string());
                Self::RequestHeaderSize(RequestHeaderSizeFilter {
                    max_size,
                    error_message,
                })
            }

            "RequestRateLimiter" => {
                let args = expect_args(&spec, 1, 1)?;
                let rate_limit = parse_number::<u32>(&spec, args[0].as_str(), 1, u32::MAX)?;
                Self::RequestRateLimiter(RequestRateLimiterFilter {
                    rate_limit,
                    limiter: Arc::new(Rate::new(Duration::from_secs(1))),
                })
            }

            "RequestSize" => {
                let args = expect_args(&spec, 1, 1)?;
                let max_size = parse_number::<u64>(&spec, args[0].as_str(), 1, u64::MAX)?;
                Self::RequestSize(RequestSizeFilter { max_size })
            }

            "RewriteLocationResponseHeader" => {
                let args = expect_args(&spec, 1, 4)?;
                let protocols_regex = match args.get(3) {
                    Some(pattern) => Some(compile_regex(&spec, pattern)?),
                    None => None,
                };
                Self::RewriteLocationResponseHeader(RewriteLocationResponseHeaderFilter {
                    strip_version_mode: args[0].as_str().into(),
                    location_header_name: args.get(1).map(|value| value.as_str().into()),
                    host_value: args.get(2).map(|value| value.as_str().into()),
                    protocols_regex,
                })
            }

            "RewritePath" => {
                let args = expect_args(&spec, 2, 2)?;
                Self::RewritePath(RewritePathFilter {
                    regex: compile_regex(&spec, &args[0])?,
                    replacement: args[1].clone(),
                })
            }

            "RewriteResponseHeader" => {
                let args = expect_args(&spec, 3, 3)?;
                Self::RewriteResponseHeader(RewriteResponseHeaderFilter {
                    header: args[0].clone(),
                    regex: compile_regex(&spec, &args[1])?,
                    replacement: args[2].clone(),
                })
            }

            "SecureHeaders" => {
                expect_args(&spec, 0, 0)?;
                Self::SecureHeaders(SecureHeadersFilter {})
            }

            "SetPath" => {
                let args = expect_args(&spec, 1, 1)?;
                Self::SetPath(SetPathFilter {
                    path: args[0].clone(),
                })
            }

            "SetRequestHeader" => Self::SetRequestHeader(SetRequestHeaderFilter {
                headers: parse_pairs(&spec)?,
            }),

            "SetRequestHostHeader" => {
                let args = expect_args(&spec, 1, 1)?;
                Self::SetRequestHostHeader(SetRequestHostHeaderFilter {
                    host: args[0].clone(),
                })
            }

            "SetResponseHeader" => Self::SetResponseHeader(SetResponseHeaderFilter {
                headers: parse_pairs(&spec)?,
            }),

            "SetStatus" => {
                let args = expect_args(&spec, 1, 1)?;
                let status = parse_number::<u16>(&spec, args[0].as_str(), 100, 599)?;
                Self::SetStatus(SetStatusFilter { status })
            }

            "StripPrefix" => {
                let args = expect_args(&spec, 1, 1)?;
                let offset = parse_number::<usize>(&spec, args[0].as_str(), 0, usize::MAX)?;
                Self::StripPrefix(StripPrefixFilter { offset })
            }

            "SaveSession" | "TokenRelay" => {
                return Err(ConfigError::new(
                    format!("filters.{}", spec.name),
                    format!(
                        "`{}` needs an authentication backend, which this gateway does not \
                         provide yet",
                        spec.name
                    ),
                ));
            }

            name => return Err(unknown_name_error("filter", name, KNOWN_FILTERS)),
        };

        Ok(filter)
    }
}

/// Parse `name, value, name, value` or the legacy `name: value` arguments.
fn parse_pairs(spec: &FilterSpec) -> Result<Vec<KeyValue<String, String>>, ConfigError> {
    if spec.args.is_empty() {
        return Err(ConfigError::new(
            format!("filters.{}", spec.name),
            "expects at least one `name, value` pair",
        ));
    }

    let mut pairs = Vec::new();
    let mut index = 0;
    while index < spec.args.len() {
        let argument = &spec.args[index];
        if let Some((name, value)) = argument.split_once(':') {
            tracing::warn!(
                "filter `{}` uses the legacy `name: value` syntax, prefer `name, value`",
                spec.name
            );
            push_pair(&mut pairs, name, value)?;
            index += 1;
        } else {
            let Some(value) = spec.args.get(index + 1) else {
                return Err(ConfigError::new(
                    format!("filters.{}", spec.name),
                    format!("`{}` has no value", argument.trim()),
                ));
            };
            push_pair(&mut pairs, argument, value)?;
            index += 2;
        }
    }

    Ok(pairs)
}

/// Parse `name, value & name, value` query parameter pairs.
fn parse_parameter_pairs(spec: &FilterSpec) -> Result<Vec<KeyValue<String, String>>, ConfigError> {
    let raw = spec.args.join(",");
    let mut pairs = Vec::new();

    for chunk in raw.split('&') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }

        let (name, value) = match chunk.split_once(',').or_else(|| chunk.split_once(':')) {
            Some(pair) => pair,
            None => {
                return Err(ConfigError::new(
                    format!("filters.{}", spec.name),
                    format!("expected `name, value` in `{chunk}`"),
                ));
            }
        };
        push_pair(&mut pairs, name, value)?;
    }

    if pairs.is_empty() {
        return Err(ConfigError::new(
            format!("filters.{}", spec.name),
            "expects at least one `name, value` pair",
        ));
    }

    Ok(pairs)
}

fn push_pair(
    pairs: &mut Vec<KeyValue<String, String>>,
    name: &str,
    value: &str,
) -> Result<(), ConfigError> {
    let name = name.trim();
    let value = value.trim();

    if name.is_empty() {
        return Err(ConfigError::new(
            "filters",
            "header or parameter name must not be empty",
        ));
    }
    if value.is_empty() {
        return Err(ConfigError::new(
            "filters",
            format!("value for `{name}` must not be empty"),
        ));
    }

    pairs.push(KeyValue::from((name, value)));
    Ok(())
}

fn require_names(spec: &FilterSpec) -> Result<Vec<String>, ConfigError> {
    if spec.args.is_empty() {
        return Err(ConfigError::new(
            format!("filters.{}", spec.name),
            "expects at least one name",
        ));
    }

    Ok(spec
        .args
        .iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect())
}

fn compile_regex(spec: &FilterSpec, pattern: &str) -> Result<Regex, ConfigError> {
    Regex::new(pattern).map_err(|error| {
        ConfigError::new(
            format!("filters.{}", spec.name),
            format!("invalid regular expression `{pattern}`: {error}"),
        )
    })
}

fn parse_number<T>(spec: &FilterSpec, raw: &str, min: T, max: T) -> Result<T, ConfigError>
where
    T: FromStr + PartialOrd + std::fmt::Display,
{
    let value = raw.trim().parse::<T>().map_err(|_| {
        ConfigError::new(
            format!("filters.{}", spec.name),
            format!(
                "`{}` is not a valid number, expected {min} to {max}",
                raw.trim()
            ),
        )
    })?;

    if value < min || value > max {
        return Err(ConfigError::new(
            format!("filters.{}", spec.name),
            format!("`{value}` is out of range, expected {min} to {max}"),
        ));
    }

    Ok(value)
}
