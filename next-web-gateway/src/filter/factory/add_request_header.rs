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
impl GatewayFilter for AddRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if let Some(request_header) = exchange.request_header() {
            for header in self.headers.iter() {
                request_header
                    .append_header(header.k.clone(), &header.v)
                    .ok();
            }
        }

        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::AddRequestHeaderFilter;
    use crate::filter::gateway_filter::GatewayFilter;
    use crate::util::key_value::KeyValue;

    fn apply(filters: &AddRequestHeaderFilter) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/api", None).unwrap();
        request.insert_header("X-Test", "a").unwrap();

        {
            let mut ctx = crate::context::RequestContext::default();
            let mut exchange = crate::server::DefaultServerWebExchange::new(
                &mut ctx,
                crate::context::HeaderAndBody::with_req_header(&mut request),
            );
            futures::executor::block_on(filters.filter(
                &mut exchange,
                &crate::handler::DefaultGatewayFilterChain::default(),
            ))
            .unwrap();
        }

        request
    }

    fn filter(pairs: &[(&str, &str)]) -> AddRequestHeaderFilter {
        AddRequestHeaderFilter {
            headers: pairs
                .iter()
                .map(|(k, v)| KeyValue {
                    k: (*k).to_owned(),
                    v: (*v).to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn adds_a_new_header() {
        let request = apply(&filter(&[("X-New", "1")]));

        assert_eq!(request.headers.get("x-new").unwrap(), "1");
    }

    #[test]
    fn appends_instead_of_replacing_an_existing_header() {
        let request = apply(&filter(&[("X-Test", "b")]));

        let values: Vec<_> = request
            .headers
            .get_all("x-test")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();

        assert_eq!(values, vec!["a", "b"]);
    }
}
