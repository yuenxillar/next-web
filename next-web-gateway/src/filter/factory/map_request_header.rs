use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;
#[derive(Debug, Clone)]
pub struct MapRequestHeaderFilter {
    pub header: KeyValue<Box<str>>,
}

#[async_trait]
impl GatewayFilter for MapRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let source = self.header.k.as_ref();
        let target = self.header.v.to_string();

        if source.eq_ignore_ascii_case(&target) {
            return chain.filter(exchange).await;
        }

        // Copy every source header value onto the target header without removing the source.
        let values = request_header
            .headers
            .get_all(source)
            .iter()
            .filter_map(|value| value.to_str().ok().map(str::to_owned))
            .collect::<Vec<_>>();

        for value in values {
            request_header.append_header(target.clone(), value).ok();
        }

        chain.filter(exchange).await
    }
}
