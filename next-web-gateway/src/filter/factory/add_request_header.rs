use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct AddRequestHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

#[async_trait]
#[async_trait]
impl GatewayFilter for AddRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        exchange.request_header().map(|request_header| {
            for header in self.headers.iter() {
                request_header
                    .append_header(header.k.clone(), &header.v)
                    .ok();
            }
        });

        chain.filter(exchange).await
    }
}
