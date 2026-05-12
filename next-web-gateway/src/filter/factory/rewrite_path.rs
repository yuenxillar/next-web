use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use regex::Regex;

use crate::util::path::set_request_path;

#[derive(Debug, Clone)]
pub struct RewritePathFilter {
    pub regex: Regex,
    pub replacement: String,
}

#[async_trait]
impl GatewayFilter for RewritePathFilter {
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

        // Rewrite only the path component and keep the original query string untouched.
        let rewritten_path = self
            .regex
            .replace(request_header.uri.path(), self.replacement.as_str())
            .to_string();

        set_request_path(request_header, &rewritten_path, query.as_deref());
        chain.filter(exchange).await
    }
}
