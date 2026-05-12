use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct CircuitBreakerFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

#[async_trait]
impl GatewayFilter for CircuitBreakerFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        self.headers.iter().for_each(|header| {
            response_header
                .append_header(header.k.clone(), header.v.as_str())
                .ok();
        });

        chain.filter(exchange).await
    }
}
