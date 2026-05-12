use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct SetRequestHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

#[async_trait]
impl GatewayFilter for SetRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        for header in self.headers.iter() {
            request_header
                .insert_header(header.k.clone(), &header.v)
                .ok();
        }

        chain.filter(exchange).await
    }
}
