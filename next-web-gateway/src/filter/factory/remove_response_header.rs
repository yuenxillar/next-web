use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RemoveResponseHeaderFilter {
    pub headers: Vec<String>,
}

#[async_trait]
impl GatewayFilter for RemoveResponseHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        // Remove every configured header name from the proxied response.
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            response_header.remove_header(&header_name);
        }

        chain.filter(exchange).await
    }
}
