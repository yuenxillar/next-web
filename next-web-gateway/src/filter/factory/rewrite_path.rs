//! `RewritePath` filter: rewrites the upstream request path with a regex.

use async_trait::async_trait;
use regex::Regex;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
use crate::util::path::set_request_path;

/// Rewrites the upstream request path, keeping the query string.
#[derive(Debug, Clone)]
pub struct RewritePathFilter {
    /// Pattern applied to the path.
    pub regex: Regex,
    /// Replacement, which may reference capture groups with `$1`.
    pub replacement: String,
}

#[async_trait]
impl GatewayFilter for RewritePathFilter {
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
        let rewritten = rewrite_path(request_header.uri.path(), &self.regex, &self.replacement);

        set_request_path(request_header, &rewritten, query.as_deref());
        chain.filter(exchange).await
    }
}

/// Replace every match, like Spring's `replaceAll`.
pub(crate) fn rewrite_path(path: &str, regex: &Regex, replacement: &str) -> String {
    regex.replace_all(path, replacement).into_owned()
}

#[cfg(test)]
mod tests {
    use super::rewrite_path;
    use regex::Regex;

    #[test]
    fn rewrite_path_replaces_all_matches() {
        let regex = Regex::new("/v1/").unwrap();
        assert_eq!(
            rewrite_path("/v1/users/v1/detail", &regex, "/v2/"),
            "/v2/users/v2/detail"
        );
    }

    #[test]
    fn rewrite_path_supports_capture_group_references() {
        let regex = Regex::new("/api/(?P<version>.+)/(.*)").unwrap();
        assert_eq!(
            rewrite_path("/api/v2/users", &regex, "/${version}/$2"),
            "/v2/users"
        );
    }
}
