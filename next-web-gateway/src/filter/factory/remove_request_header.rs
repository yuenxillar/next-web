use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct RemoveRequestHeaderFilter {
    pub headers: Vec<String>,
}

#[async_trait]
impl GatewayFilter for RemoveRequestHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        // Remove every configured header name from the proxied request.
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            request_header.remove_header(&header_name);
        }

        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::RemoveRequestHeaderFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn apply(names: &[&str]) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/api", None).unwrap();
        request.insert_header("X-Test", "1").unwrap();
        request.insert_header("X-Keep", "2").unwrap();
        let filter = RemoveRequestHeaderFilter {
            headers: names.iter().map(|name| (*name).to_owned()).collect(),
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
    fn removes_the_named_header_only() {
        let request = apply(&["X-Test"]);

        assert!(request.headers.get("x-test").is_none());
        assert_eq!(request.headers.get("x-keep").unwrap(), "2");
    }

    #[test]
    fn matches_the_header_name_case_insensitively() {
        let request = apply(&["x-test"]);

        assert!(request.headers.get("x-test").is_none());
    }
}
