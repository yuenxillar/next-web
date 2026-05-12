use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct SetResponseHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

#[async_trait]
impl GatewayFilter for SetResponseHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        for header in self.headers.iter() {
            response_header
                .insert_header(header.k.clone(), &header.v)
                .ok();
        }

        chain.filter(exchange).await
    }
}
