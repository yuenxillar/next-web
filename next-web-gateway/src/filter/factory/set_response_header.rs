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

#[cfg(test)]
mod tests {
    use pingora::http::ResponseHeader;

    use super::SetResponseHeaderFilter;
    use crate::filter::gateway_filter::GatewayFilter;
    use crate::util::key_value::KeyValue;

    fn apply(pairs: &[(&str, &str)]) -> ResponseHeader {
        let mut response = ResponseHeader::build(200, None).unwrap();
        response.append_header("X-Test", "a").unwrap();
        response.append_header("X-Test", "b").unwrap();
        let filter = SetResponseHeaderFilter {
            headers: pairs
                .iter()
                .map(|(k, v)| KeyValue {
                    k: (*k).to_owned(),
                    v: (*v).to_owned(),
                })
                .collect(),
        };

        {
            let mut ctx = crate::context::RequestContext::default();
            let mut exchange = crate::server::DefaultServerWebExchange::new(
                &mut ctx,
                crate::context::HeaderAndBody::with_resp_header(&mut response),
            );
            futures::executor::block_on(filter.filter(
                &mut exchange,
                &crate::handler::DefaultGatewayFilterChain::default(),
            ))
            .unwrap();
        }

        response
    }

    #[test]
    fn collapses_duplicate_values_into_one() {
        let response = apply(&[("X-Test", "c")]);

        assert_eq!(response.headers.get_all("x-test").iter().count(), 1);
        assert_eq!(response.headers.get("x-test").unwrap(), "c");
    }
}
