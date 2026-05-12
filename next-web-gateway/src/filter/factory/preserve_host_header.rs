use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct PreserveHostHeaderFilter {}

#[async_trait]
impl GatewayFilter for PreserveHostHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        if request_header.headers.contains_key("host") {
            return chain.filter(exchange).await;
        }

        let authority = request_header
            .uri
            .authority()
            .map(|authority| authority.as_str().to_string());

        if let Some(authority) = authority {
            // Restore the downstream host header when only the URI authority is available.
            request_header.insert_header("Host", authority).ok();
        }

        chain.filter(exchange).await
    }
}
