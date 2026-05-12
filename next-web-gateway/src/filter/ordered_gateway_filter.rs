use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::GatewayError,
    filter::{gateway_filter::GatewayFilter, gateway_filter_chain::GatewayFilterChain},
    server::ServerWebExchange,
    Ordered,
};

pub struct OrderedGatewayFilter {
    pub delegate: Arc<dyn GatewayFilter>,
    pub order: i32,
}

impl OrderedGatewayFilter {
    pub fn new(delegate: Arc<dyn GatewayFilter>, order: i32) -> Self {
        Self { delegate, order }
    }
}

#[async_trait]
impl GatewayFilter for OrderedGatewayFilter {
    async fn filter(
        &self,
        exchange: &mut dyn ServerWebExchange,
        chain: &dyn GatewayFilterChain,
    ) -> Result<(), GatewayError> {
        self.delegate.filter(exchange, chain).await
    }
}

impl Ordered for OrderedGatewayFilter {
    fn order(&self) -> i32 {
        self.order
    }
}
