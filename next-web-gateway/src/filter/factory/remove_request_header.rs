use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RemoveRequestHeaderFilter {
    pub headers: Vec<String>,
}

#[async_trait]
impl GatewayFilter for RemoveRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        // Remove every configured header name from the proxied request.
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            request_header.remove_header(&header_name);
        }

        chain.filter(exchange).await
    }
}
