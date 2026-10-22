use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct SetRequestHostHeaderFilter {
    pub host: String,
}

#[async_trait]
impl GatewayFilter for SetRequestHostHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        // The proxy rewrites `Host` from the upstream authority by default, so
        // the filter records the desired value on the context and the proxy
        // honours it.
        exchange.request_context().upstream_host_override = Some(self.host.clone());

        chain.filter(exchange).await
    }
}
