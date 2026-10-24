use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::{set_request_path, substitute_variables};

/// Replaces the upstream request path, substituting `{variable}` captures.
#[derive(Debug, Clone)]
pub struct SetPathFilter {
    /// Path template, optionally carrying a query string.
    pub path: String,
}

#[async_trait]
impl GatewayFilter for SetPathFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let variables = exchange.request_context().path_variables.clone();
        let template = substitute_variables(&self.path, &variables);

        let request_header = match exchange.request_header() {
            Some(request_header) => request_header,
            None => return chain.filter(exchange).await,
        };

        let original_query = request_header.uri.query().map(str::to_owned);
        let (path, query) = match template.split_once('?') {
            Some((path, query)) => (path.to_owned(), Some(query.to_owned())),
            None => (template, original_query),
        };

        // Replace the path while preserving the original query unless the
        // template carries a new one.
        set_request_path(request_header, &path, query.as_deref());
        chain.filter(exchange).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use pingora::http::RequestHeader;

    use super::SetPathFilter;
    use crate::filter::gateway_filter::GatewayFilter;

    fn apply(template: &str, variables: &[(&str, &str)]) -> RequestHeader {
        let mut request = RequestHeader::build("GET", b"/old/path?a=1", None).unwrap();
        let filter = SetPathFilter {
            path: template.to_owned(),
        };
        let variables: HashMap<String, String> = variables
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();

        {
            let mut ctx = crate::context::RequestContext {
                path_variables: variables,
                ..crate::context::RequestContext::default()
            };
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
    fn substitutes_captured_variables_and_keeps_the_query() {
        let request = apply("/new/{id}", &[("id", "42")]);

        assert_eq!(request.uri.path(), "/new/42");
        assert_eq!(request.uri.query(), Some("a=1"));
    }

    #[test]
    fn a_query_in_the_template_replaces_the_original_one() {
        let request = apply("/new?b=2", &[]);

        assert_eq!(request.uri.path(), "/new");
        assert_eq!(request.uri.query(), Some("b=2"));
    }
}
