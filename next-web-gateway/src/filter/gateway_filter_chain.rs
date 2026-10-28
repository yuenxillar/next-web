use async_trait::async_trait;

use crate::{error::GatewayError, server::ServerWebExchange};

#[async_trait]
pub trait GatewayFilterChain: Sync {
    async fn filter(&self, exchange: &mut dyn ServerWebExchange) -> Result<(), GatewayError>;
}
