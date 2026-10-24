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

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::SetRequestHeaderFilter;
    use crate::filter::gateway_filter::GatewayFilter;
    use crate::util::key_value::KeyValue;

    fn apply(pairs: &[(&str, &str)]) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/api", None).unwrap();
        request.append_header("X-Test", "a").unwrap();
        let filter = SetRequestHeaderFilter {
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
                crate::context::HeaderAndBody::with_req_header(&mut request),
            );
            futures::executor::block_on(filter.filter(
                &mut exchange,
                &crate::handler::DefaultGatewayFilterChain::default(),
            ))
            .unwrap();
        }

        request
    }

    #[test]
    fn replaces_every_existing_value() {
        let request = apply(&[("X-Test", "b")]);

        assert_eq!(request.headers.get_all("x-test").iter().count(), 1);
        assert_eq!(request.headers.get("x-test").unwrap(), "b");
    }

    #[test]
    fn adds_a_header_that_did_not_exist() {
        let request = apply(&[("X-New", "1")]);

        assert_eq!(request.headers.get("x-new").unwrap(), "1");
    }
}
