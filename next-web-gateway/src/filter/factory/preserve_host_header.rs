//! `PreserveHostHeader` filter: keeps the downstream `Host` on the upstream request.
//!
//! The proxy rewrites `Host` to the upstream authority by default. This filter
//! asks the proxy to keep the client's value instead.

use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;

/// Marks the downstream `Host` as authoritative for this request.
#[derive(Debug, Clone, Default)]
pub struct PreserveHostHeaderFilter {}

#[async_trait]
impl GatewayFilter for PreserveHostHeaderFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        exchange.request_context().preserve_host = true;

        // Fall back to the request target's authority when there is no header.
        if exchange.request_context().original_host.is_none() {
            let host = exchange.request_header().and_then(|request_header| {
                request_header
                    .headers
                    .get("host")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned)
                    .or_else(|| {
                        request_header
                            .uri
                            .authority()
                            .map(|authority| authority.as_str().to_owned())
                    })
            });

            if let Some(host) = host {
                exchange.request_context().original_host = Some(host);
            }
        }

        chain.filter(exchange).await
    }
}
