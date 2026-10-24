use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RequestSizeFilter {
    // byte
    pub max_size: u64,
}

#[async_trait]
impl GatewayFilter for RequestSizeFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        if self.max_size == 0 {
            return chain.filter(exchange).await;
        }

        // Reject a request that already advertises a body larger than the limit;
        // a chunked body is caught while it streams in the proxy.
        let advertised = request_header
            .headers
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        if let Some(size) = advertised
            && size > self.max_size
        {
            return exchange
                .request_context()
                .respond_with_text(413, Vec::new(), "Request size exceeded")
                .map_err(|_| GatewayError::ServerRejectsRequest);
        }

        chain.filter(exchange).await
    }
}
