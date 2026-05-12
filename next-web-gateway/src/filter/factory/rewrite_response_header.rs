use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use regex::Regex;

use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct RewriteResponseHeaderFilter {
    pub header: (KeyValue<String, String>, Option<Regex>),
}

#[async_trait]
impl GatewayFilter for RewriteResponseHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        if let Some(value) = response_header.headers.get(&self.header.0.k) {
            if let Some(regex) = &self.header.1 {
                if regex.is_match(value.to_str().unwrap_or_default()) {
                    let _ = response_header
                        .insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
                }
            } else {
                let _ = response_header
                    .insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
            }
        }

        chain.filter(exchange).await
    }
}
