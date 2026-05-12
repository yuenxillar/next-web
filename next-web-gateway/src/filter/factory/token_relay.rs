use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone, Default)]
pub struct TokenRelayFilter {}

#[async_trait]
impl GatewayFilter for TokenRelayFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let bearer_token = request_header
            .headers
            .get_all("authorization")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| value.len() > 7 && value[..7].eq_ignore_ascii_case("Bearer "))
            .map(str::to_string);

        let Some(bearer_token) = bearer_token else {
            return chain.filter(exchange).await;
        };

        // Normalize the proxied Authorization header to a single Bearer token value.
        request_header
            .insert_header("Authorization", bearer_token)
            .ok();

        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::TokenRelayFilter;
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

    #[test]
    fn relays_incoming_bearer_token() {
        let mut request = RequestHeader::build("GET", b"/resource", None).unwrap();
        request
            .append_header("Authorization", "Bearer access-token")
            .unwrap();

        let filter = TokenRelayFilter::default();
        let mut ctx = test_ctx();
        let mut exchange = crate::server::DefaultServerWebExchange::new(&mut ctx, crate::context::HeaderAndBody::with_req_header(&mut request));

        futures::executor::block_on(filter.filter(&mut exchange, &crate::handler::DefaultGatewayFilterChain::default())).unwrap();
        drop(exchange);

        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer access-token")
        );
    }

    #[test]
    fn ignores_non_bearer_authorization_headers() {
        let mut request = RequestHeader::build("GET", b"/resource", None).unwrap();
        request
            .insert_header("Authorization", "Basic ZGVtbzpwYXNz")
            .unwrap();

        let filter = TokenRelayFilter::default();
        let mut ctx = test_ctx();
        let mut exchange = crate::server::DefaultServerWebExchange::new(&mut ctx, crate::context::HeaderAndBody::with_req_header(&mut request));

        futures::executor::block_on(filter.filter(&mut exchange, &crate::handler::DefaultGatewayFilterChain::default())).unwrap();
        drop(exchange);

        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Basic ZGVtbzpwYXNz")
        );
    }
}
