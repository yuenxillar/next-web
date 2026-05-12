use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct SecureHeadersFilter;

#[async_trait]
impl GatewayFilter for SecureHeadersFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        // Add the standard hardening headers without overriding explicit route responses.
        insert_if_missing(response_header, "X-Content-Type-Options", "nosniff");
        insert_if_missing(response_header, "X-Frame-Options", "DENY");
        insert_if_missing(response_header, "Referrer-Policy", "no-referrer");
        insert_if_missing(response_header, "X-XSS-Protection", "1; mode=block");
        insert_if_missing(
            response_header,
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        );
        insert_if_missing(
            response_header,
            "Permissions-Policy",
            "geolocation=(), microphone=(), camera=()",
        );

        chain.filter(exchange).await
    }
}

fn insert_if_missing(response_header: &mut pingora::http::ResponseHeader, name: &str, value: &str) {
    if !response_header.headers.contains_key(name) {
        response_header.insert_header(name.to_string(), value).ok();
    }
}
