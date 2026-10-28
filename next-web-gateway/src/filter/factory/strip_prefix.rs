use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::{set_request_path, strip_prefix_segments};

#[derive(Debug, Clone)]
pub struct StripPrefixFilter {
    pub offset: usize,
}

#[async_trait]
impl GatewayFilter for StripPrefixFilter {
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

        // Drop the configured number of leading path segments and preserve the query string.
        let rewritten_path = strip_prefix_segments(request_header.uri.path(), self.offset);
        set_request_path(request_header, &rewritten_path, query.as_deref());
        chain.filter(exchange).await
    }
}
