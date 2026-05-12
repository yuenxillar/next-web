use async_trait::async_trait;

use crate::error::GatewayError;
use crate::filter::gateway_filter::GatewayFilter;
use crate::filter::gateway_filter_chain::GatewayFilterChain;
use crate::server::ServerWebExchange;
#[derive(Debug, Clone)]
pub struct SaveSessionFilter {}

#[async_trait]
impl GatewayFilter for SaveSessionFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        let session = exchange.request_context().session.clone();
        let response_header = match exchange.response_header() {
            Some(response_header) => response_header,
            None => return chain.filter(exchange).await,
        };

        if let Some(session) = session {
            // Persist the gateway session as a response cookie when one exists on the context.
            response_header
                .append_header(
                    "Set-Cookie",
                    format!("session_id={session}; Path=/; HttpOnly; SameSite=Lax"),
                )
                .ok();
        }

        chain.filter(exchange).await
    }
}
