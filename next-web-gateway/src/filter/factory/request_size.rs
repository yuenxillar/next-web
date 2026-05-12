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

        if let Some(content_length) = request_header.headers.get("content-length") {
            if let Ok(content_length) = content_length.to_str() {
                if let Ok(size) = content_length.parse::<u64>() {
                    // Reject requests that already advertise a body larger than the configured limit.
                    if size > self.max_size {
                        return exchange
                            .request_context()
                            .respond_with_text(413, Vec::new(), "Request size exceeded")
                            .map_err(|_| GatewayError::ServerRejectsRequest);
                    }
                }
            }
        }

        chain.filter(exchange).await
    }
}
