//! `SecureHeaders` filter: adds the Spring Cloud Gateway hardening header set.
//!
//! The configured values win: an upstream that weakens `X-Frame-Options` or
//! drops `Strict-Transport-Security` cannot undo the policy.

use async_trait::async_trait;
use pingora::http::ResponseHeader;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// The hardening headers applied by the filter, in the Spring Cloud Gateway set.
pub(crate) const SECURE_HEADERS: &[(&str, &str)] = &[
    ("X-XSS-Protection", "1 ; mode=block"),
    ("Strict-Transport-Security", "max-age=631138519"),
    ("X-Frame-Options", "DENY"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    (
        "Content-Security-Policy",
        "default-src 'self' https:; font-src 'self' https: data:; img-src 'self' https: data:; \
         object-src 'none'; script-src https:; style-src 'self' https: 'unsafe-inline'",
    ),
    ("X-Download-Options", "noopen"),
    ("X-Permitted-Cross-Domain-Policies", "none"),
];

/// Adds the hardening headers to every response.
#[derive(Debug, Clone, Default)]
pub struct SecureHeadersFilter {}

#[async_trait]
impl GatewayFilter for SecureHeadersFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        if let Some(response_header) = exchange.response_header() {
            apply_secure_headers(response_header);
        }

        chain.filter(exchange).await
    }
}

/// Overwrite the hardening headers, whatever the upstream sent.
pub(crate) fn apply_secure_headers(response_header: &mut ResponseHeader) {
    for (name, value) in SECURE_HEADERS {
        response_header.insert_header(*name, *value).ok();
    }
}

#[cfg(test)]
mod tests {
    use pingora::http::ResponseHeader;

    use super::{apply_secure_headers, SECURE_HEADERS};

    #[test]
    fn secure_headers_are_all_applied() {
        let mut response = ResponseHeader::build(200, None).unwrap();
        apply_secure_headers(&mut response);

        for (name, value) in SECURE_HEADERS {
            assert_eq!(
                response
                    .headers
                    .get(*name)
                    .and_then(|header| header.to_str().ok()),
                Some(*value),
                "missing or wrong value for {name}"
            );
        }
    }

    #[test]
    fn secure_headers_overrides_weakened_upstream_values() {
        let mut response = ResponseHeader::build(200, None).unwrap();
        response
            .insert_header("X-Frame-Options", "ALLOWALL")
            .unwrap();
        response.insert_header("Strict-Transport-Security", "max-age=0").unwrap();

        apply_secure_headers(&mut response);

        assert_eq!(
            response
                .headers
                .get("x-frame-options")
                .and_then(|value| value.to_str().ok()),
            Some("DENY")
        );
        assert_eq!(
            response
                .headers
                .get("strict-transport-security")
                .and_then(|value| value.to_str().ok()),
            Some("max-age=631138519")
        );
    }
}
