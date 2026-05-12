use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use std::collections::HashSet;

use form_urlencoded::{parse, Serializer};
use pingora::http::RequestHeader;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RemoveRequestParameterFilter {
    pub names: Vec<String>,
}

#[async_trait]
impl GatewayFilter for RemoveRequestParameterFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if self.names.is_empty() {
            return chain.filter(exchange).await;
        }

        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let query = match request_header.uri.query() {
            Some(query) if !query.is_empty() => query,
            _ => return chain.filter(exchange).await,
        };

        let new_query = {
            let names: HashSet<&str> = self.names.iter().map(String::as_str).collect();
            let mut removed_any = false;
            let mut serializer = Serializer::new(String::with_capacity(query.len()));

            for (name, value) in parse(query.as_bytes()) {
                if names.contains(name.as_ref()) {
                    removed_any = true;
                    continue;
                }

                serializer.append_pair(name.as_ref(), value.as_ref());
            }

            if !removed_any {
                None
            } else {
                Some(serializer.finish())
            }
        };

        let Some(new_query) = new_query else {
            return chain.filter(exchange).await;
        };

        rewrite_request_uri(request_header, &new_query);

        chain.filter(exchange).await
    }
}

fn rewrite_request_uri(request_header: &mut RequestHeader, query: &str) {
    let uri = &request_header.uri;
    let path = uri.path();
    let scheme = uri.scheme().map(|value| value.as_str()).unwrap_or("");
    let authority = uri.authority().map(|value| value.as_str()).unwrap_or("");

    let new_uri = build_uri_string(scheme, authority, path, query);

    match new_uri.parse() {
        Ok(uri) => request_header.set_uri(uri),
        Err(error) => warn!(
            target: "gateway_filter",
            "Failed to parse modified URI: {}, original_uri: {}",
            error,
            request_header.uri
        ),
    }
}

fn build_uri_string(scheme: &str, authority: &str, path: &str, query: &str) -> String {
    let mut uri_string = String::new();

    if !scheme.is_empty() {
        uri_string.push_str(scheme);
        uri_string.push_str("://");
    }

    if !authority.is_empty() {
        uri_string.push_str(authority);
    }

    uri_string.push_str(path);

    if !query.is_empty() {
        uri_string.push('?');
        uri_string.push_str(query);
    }

    uri_string
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::RemoveRequestParameterFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn test_ctx() -> crate::context::RequestContext {
        crate::context::RequestContext {
            fallback_id: None,
            route_id: None,
            original_request_path: None,
            buffer_response_body: false,
            response_body_buffer: Vec::new(),
            local_response_cache_manager: None,
            local_response_cache_request: None,
            pending_local_response_cache: None,
            session: None,
            direct_response: None,
        }
    }

    fn apply_filter(raw_path: &[u8], names: &[&str]) -> RequestHeader {
        let mut request = RequestHeader::build("GET", raw_path, None).unwrap();
        let filter = RemoveRequestParameterFilter {
            names: names.iter().map(|name| name.to_string()).collect(),
        };
        let mut ctx = test_ctx();
        let mut exchange = crate::server::DefaultServerWebExchange::new(&mut ctx, crate::context::HeaderAndBody::with_req_header(&mut request));

        futures::executor::block_on(filter.filter(&mut exchange, &crate::handler::DefaultGatewayFilterChain::default())).unwrap();
        drop(exchange);

        request
    }

    #[test]
    fn removes_all_matching_parameters_and_drops_empty_query() {
        let request = apply_filter(b"/search?token=1&token=2", &["token"]);

        assert_eq!(request.uri.path(), "/search");
        assert_eq!(request.uri.query(), None);
        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search")
        );
    }

    #[test]
    fn preserves_query_encoding_when_removing_other_parameters() {
        let request = apply_filter(
            b"/search?keep=1&encoded=hello%20world&plus=a%2Bb&drop=gone",
            &["drop"],
        );

        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search?keep=1&encoded=hello+world&plus=a%2Bb")
        );
    }

    #[test]
    fn leaves_uri_unchanged_when_no_parameter_matches() {
        let request = apply_filter(b"/search?keep=1&encoded=hello%20world", &["drop"]);

        assert_eq!(
            request.uri.path_and_query().map(|value| value.as_str()),
            Some("/search?keep=1&encoded=hello%20world")
        );
    }
}
