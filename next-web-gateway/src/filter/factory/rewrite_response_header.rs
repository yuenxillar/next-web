//! `RewriteResponseHeader` filter: regex rewrite of a response header value.
//!
//! Configuration uses the Spring Cloud Gateway argument order:
//! `RewriteResponseHeader=Name, regexp, replacement`.

use async_trait::async_trait;
use regex::Regex;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// Rewrites every value of a response header with a regular expression.
#[derive(Debug, Clone)]
pub struct RewriteResponseHeaderFilter {
    /// Header name to rewrite.
    pub header: String,
    /// Pattern applied to the current value.
    pub regex: Regex,
    /// Replacement, which may reference capture groups with `$1`.
    pub replacement: String,
}

#[async_trait]
impl GatewayFilter for RewriteResponseHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        // Every value of a multi-valued header (`Set-Cookie`, `Warning`,
        // `Link`) is rewritten, not just the first one.
        let values: Vec<String> = response_header
            .headers
            .get_all(&self.header)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(str::to_owned)
            .collect();

        if values.is_empty() {
            return chain.filter(exchange).await;
        }

        let rewritten: Vec<String> = values
            .iter()
            .map(|value| rewrite_value(value, &self.regex, &self.replacement))
            .collect();

        if rewritten != values {
            response_header.remove_header(&self.header);
            for value in rewritten {
                response_header
                    .append_header(self.header.clone(), value)
                    .ok();
            }
        }

        chain.filter(exchange).await
    }
}

/// Rewrite every match inside an existing header value.
pub(crate) fn rewrite_value(value: &str, regex: &Regex, replacement: &str) -> String {
    regex.replace_all(value, replacement).into_owned()
}

#[cfg(test)]
mod tests {
    use super::rewrite_value;
    use regex::Regex;

    #[test]
    fn rewrite_response_header_keeps_the_untouched_parts_of_the_value() {
        let regex = Regex::new("abc").unwrap();
        assert_eq!(
            rewrite_value("prefix abc suffix", &regex, "abc-kept"),
            "prefix abc-kept suffix"
        );
    }

    #[test]
    fn rewrite_response_header_supports_capture_group_references() {
        let regex = Regex::new(r"abc-(\w+)").unwrap();
        assert_eq!(rewrite_value("x abc-123 y", &regex, "id:$1"), "x id:123 y");
    }

    #[test]
    fn rewrite_response_header_replaces_every_match() {
        let regex = Regex::new("http://").unwrap();
        assert_eq!(
            rewrite_value("http://a, http://b", &regex, "https://"),
            "https://a, https://b"
        );
    }
}
