use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::{join_paths, set_request_path};

#[derive(Debug, Clone)]
pub struct PrefixPathFilter {
    pub path: Box<str>,
}

#[async_trait]
impl GatewayFilter for PrefixPathFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let query = request_header.uri.query().map(str::to_owned);

        // Prefix the configured path while preserving the original query string.
        let rewritten_path = join_paths(self.path.as_ref(), request_header.uri.path());
        set_request_path(request_header, &rewritten_path, query.as_deref());
        
        
        chain.filter(exchange).await
    }
}
