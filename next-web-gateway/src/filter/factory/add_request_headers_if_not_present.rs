use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct AddRequestHeaderIfNotPresentFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

#[async_trait]
impl GatewayFilter for AddRequestHeaderIfNotPresentFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if let Some(request_header) = exchange.request_header() {
            for header in &self.headers {
                if !request_header.headers.contains_key(header.k.as_str()) {
                    request_header
                        .insert_header(header.k.clone(), header.v.as_str())
                        .ok();
                }
            }
        }

        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::AddRequestHeaderIfNotPresentFilter;
    use crate::filter::gateway_filter::GatewayFilter;
    use crate::util::key_value::KeyValue;

    fn filter(pairs: &[(&str, &str)]) -> AddRequestHeaderIfNotPresentFilter {
        AddRequestHeaderIfNotPresentFilter {
            headers: pairs
                .iter()
                .map(|(k, v)| KeyValue {
                    k: (*k).to_owned(),
                    v: (*v).to_owned(),
                })
                .collect(),
        }
    }

    fn apply(filters: &AddRequestHeaderIfNotPresentFilter) -> RequestHeader {
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

    #[test]
    fn keeps_an_existing_header_untouched() {
        let request = apply(&filter(&[("X-Test", "ignored")]));

        assert_eq!(request.headers.get("x-test").unwrap(), "a");
        assert_eq!(request.headers.get_all("x-test").iter().count(), 1);
    }

    #[test]
    fn adds_the_header_when_it_is_absent() {
        let request = apply(&filter(&[("X-Other", "2")]));

        assert_eq!(request.headers.get("x-other").unwrap(), "2");
    }
}
