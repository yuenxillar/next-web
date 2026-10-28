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

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::MapRequestHeaderFilter;
    use crate::filter::gateway_filter::GatewayFilter;
    use crate::util::key_value::KeyValue;

    fn filter(from: &str, to: &str) -> MapRequestHeaderFilter {
        MapRequestHeaderFilter {
            header: KeyValue {
                k: from.into(),
                v: to.into(),
            },
        }
    }

    fn apply(filters: &MapRequestHeaderFilter) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/api", None).unwrap();
        request.append_header("X-Src", "one").unwrap();
        request.append_header("X-Src", "two").unwrap();
        request.insert_header("X-Dst", "zero").unwrap();

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
    fn copies_every_value_and_keeps_the_source() {
        let request = apply(&filter("X-Src", "X-Dst"));

        let values: Vec<_> = request
            .headers
            .get_all("x-dst")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();

        assert_eq!(values, vec!["zero", "one", "two"]);
        assert_eq!(request.headers.get_all("x-src").iter().count(), 2);
    }

    #[test]
    fn does_nothing_when_the_source_is_missing() {
        let request = apply(&filter("X-Missing", "X-Dst"));

        let values: Vec<_> = request
            .headers
            .get_all("x-dst")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .collect();

        assert_eq!(values, vec!["zero"]);
    }
}
