use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::{join_paths, set_request_path};

#[derive(Debug, Clone)]
pub struct PrefixPathFilter {
    pub path: Box<str>,
}

#[async_trait]
impl GatewayFilter for PrefixPathFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let query = request_header.uri.query().map(str::to_owned);

        // Prefix the configured path while preserving the original query string.
        let rewritten_path = join_paths(self.path.as_ref(), request_header.uri.path());
        set_request_path(request_header, &rewritten_path, query.as_deref());

        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::RequestHeader;

    use super::PrefixPathFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn apply(prefix: &str) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/api/users?x=1", None).unwrap();
        let filter = PrefixPathFilter {
            path: prefix.into(),
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
    fn prepends_the_prefix_and_keeps_the_query() {
        let request = apply("/root");

        assert_eq!(request.uri.path(), "/root/api/users");
        assert_eq!(request.uri.query(), Some("x=1"));
    }

    #[test]
    fn tolerates_a_trailing_slash_on_the_prefix() {
        let request = apply("/root/");

        assert_eq!(request.uri.path(), "/root/api/users");
    }
}
