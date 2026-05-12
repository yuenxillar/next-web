use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::set_request_path;

#[derive(Debug, Clone)]
pub struct SetPathFilter {
    pub path: String,
}

#[async_trait]
impl GatewayFilter for SetPathFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let original_query = request_header.uri.query().map(str::to_owned);
        let (path, query) = match self.path.split_once('?') {
            Some((path, query)) => (path.to_string(), Some(query.to_string())),
            None => (self.path.clone(), original_query),
        };

        // Replace the path while preserving the original query unless a new one is configured.
        set_request_path(request_header, &path, query.as_deref());
        chain.filter(exchange).await
    }
}
