use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::GatewayError,
    filter::{gateway_filter::GatewayFilter, gateway_filter_chain::GatewayFilterChain},
    server::ServerWebExchange,
};

pub struct DefaultGatewayFilterChain {
    index: usize,
    filters: Vec<Arc<dyn GatewayFilter>>,
}

impl DefaultGatewayFilterChain {
    pub fn new(filters: Vec<Arc<dyn GatewayFilter>>) -> Self {
        Self { index: 0, filters }
    }

    pub fn from_parent(value: DefaultGatewayFilterChain) -> Self {
        Self {
            index: value.index,
            filters: value.filters,
        }
    }

    pub fn filters(&self) -> &[Arc<dyn GatewayFilter>] {
        &self.filters
    }
}

impl Default for DefaultGatewayFilterChain {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[async_trait]
impl GatewayFilterChain for DefaultGatewayFilterChain {
    async fn filter(&self, exchange: &mut dyn ServerWebExchange) -> Result<(), GatewayError> {
        let wrapper = DefaultGatewayFilterChainWrapper {
            chain: self,
            index: self.index,
        };
        wrapper.filter(exchange).await
    }
}

struct DefaultGatewayFilterChainWrapper<'a> {
    chain: &'a DefaultGatewayFilterChain,
    index: usize,
}

#[async_trait]
impl GatewayFilterChain for DefaultGatewayFilterChainWrapper<'_> {
    async fn filter(&self, exchange: &mut dyn ServerWebExchange) -> Result<(), GatewayError> {
        if self.index < self.chain.filters.len() {
            if let Some(filter) = self.chain.filters.get(self.index) {
                let chain = DefaultGatewayFilterChainWrapper {
                    chain: self.chain,
                    index: self.index + 1,
                };
                return filter.filter(exchange, &chain).await;
            }
        }

        // complete
        Ok(())
    }
}
