use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::{set_request_path, substitute_variables};

/// Replaces the upstream request path, substituting `{variable}` captures.
#[derive(Debug, Clone)]
pub struct SetPathFilter {
    /// Path template, optionally carrying a query string.
    pub path: String,
}

#[async_trait]
impl GatewayFilter for SetPathFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let variables = exchange.request_context().path_variables.clone();
        let template = substitute_variables(&self.path, &variables);

        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let original_query = request_header.uri.query().map(str::to_owned);
        let (path, query) = match template.split_once('?') {
            Some((path, query)) => (path.to_string(), Some(query.to_string())),
            None => (template, original_query),
        };

        // Replace the path while preserving the original query unless the
        // template carries a new one.
        set_request_path(request_header, &path, query.as_deref());
        chain.filter(exchange).await
    }
}
