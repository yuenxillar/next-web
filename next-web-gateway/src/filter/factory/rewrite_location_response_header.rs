use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use regex::Regex;

use crate::util::path::build_path_and_query;

#[derive(Debug, Clone)]
pub struct RewriteLocationResponseHeaderFilter {
    pub strip_version_mode: Box<str>,
    pub location_header_name: Option<Box<str>>,
    pub host_value: Option<Box<str>>,
    pub protocols_regex: Option<Regex>,
}

#[async_trait]
impl GatewayFilter for RewriteLocationResponseHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let original_request_path = exchange.request_context().original_request_path.clone();
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        let header_name = self.location_header_name.as_deref().unwrap_or("Location");
        let location = match response_header.headers.get(header_name) {
            Some(value) => match value.to_str() {
                Ok(value) => value.to_string(),
                Err(_) => return chain.filter(exchange).await,
            },
            None => return chain.filter(exchange).await,
        };

        let parsed_location = ParsedLocation::parse(&location);

        if let (Some(regex), Some(scheme)) =
            (&self.protocols_regex, parsed_location.scheme.as_deref())
        {
            if !regex.is_match(scheme) {
                return chain.filter(exchange).await;
            }
        }

        let rewritten_path = match self.strip_version_mode.to_ascii_uppercase().as_str() {
            "ALWAYS_STRIP" => strip_version_prefix(&parsed_location.path),
            "AS_IN_REQUEST" => {
                let request_has_version = original_request_path
                    .as_deref()
                    .map(has_version_prefix)
                    .unwrap_or(false);

                if request_has_version {
                    parsed_location.path.clone()
                } else {
                    strip_version_prefix(&parsed_location.path)
                }
            }
            _ => parsed_location.path.clone(),
        };

        let path_and_query =
            build_path_and_query(&rewritten_path, parsed_location.query.as_deref());
        let rewritten_location = build_location(
            parsed_location.scheme.as_deref(),
            self.host_value
                .as_deref()
                .filter(|value| !value.is_empty())
                .or(parsed_location.authority.as_deref()),
            &path_and_query,
        );

        // Rewrite the header in place after all path and host adjustments are resolved.
        response_header
            .insert_header(header_name.to_string(), rewritten_location)
            .ok();

        chain.filter(exchange).await
    }
}

fn build_location(scheme: Option<&str>, authority: Option<&str>, path_and_query: &str) -> String {
    let mut value = String::new();

    if let Some(scheme) = scheme {
        value.push_str(scheme);
        value.push_str("://");
    }

    if let Some(authority) = authority {
        value.push_str(authority);
    }

    value.push_str(path_and_query);
    value
}

fn has_version_prefix(path: &str) -> bool {
    let path = path.split('?').next().unwrap_or(path);
    let first_segment = path.trim_start_matches('/').split('/').next().unwrap_or("");
    is_version_segment(first_segment)
}

fn strip_version_prefix(path: &str) -> String {
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    let first_segment = segments.next();

    if first_segment.is_some_and(is_version_segment) {
        let remaining = segments.collect::<Vec<_>>();
        if remaining.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", remaining.join("/"))
        }
    } else {
        path.to_string()
    }
}

fn is_version_segment(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    bytes.len() > 1
        && matches!(bytes.first(), Some(b'v' | b'V'))
        && bytes[1..].iter().all(u8::is_ascii_digit)
}

struct ParsedLocation {
    scheme: Option<String>,
    authority: Option<String>,
    path: String,
    query: Option<String>,
}

impl ParsedLocation {
    fn parse(value: &str) -> Self {
        let (scheme, remainder) = if let Some((scheme, remainder)) = value.split_once("://") {
            (Some(scheme.to_string()), remainder)
        } else {
            (None, value)
        };

        let (authority, path_and_query) = if scheme.is_some() {
            match remainder.find('/') {
                Some(index) => (Some(remainder[..index].to_string()), &remainder[index..]),
                None => (Some(remainder.to_string()), "/"),
            }
        } else {
            (None, remainder)
        };

        let (path, query) = match path_and_query.split_once('?') {
            Some((path, query)) => (path.to_string(), Some(query.to_string())),
            None => (path_and_query.to_string(), None),
        };

        Self {
            scheme,
            authority,
            path,
            query,
        }
    }
}
