//! The filter chain: each filter either continues the chain or aborts.

use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    error::GatewayError,
    filter::{gateway_filter::GatewayFilter, gateway_filter_chain::GatewayFilterChain},
    server::ServerWebExchange,
};

/// Runs a list of filters in order.
///
/// The list is built once per route and phase, so a request only borrows it: no
/// per-request filtering, cloning or `Arc` allocation on the hot path.
pub(crate) struct DefaultGatewayFilterChain {
    filters: Arc<[Arc<dyn GatewayFilter>]>,
}

impl DefaultGatewayFilterChain {
    /// Build a chain from the filters of one phase.
    pub(crate) fn new(filters: Arc<[Arc<dyn GatewayFilter>]>) -> Self {
        Self { filters }
    }
}

impl Default for DefaultGatewayFilterChain {
    fn default() -> Self {
        Self::new(Arc::from(Vec::new()))
    }
}

#[async_trait]
impl GatewayFilterChain for DefaultGatewayFilterChain {
    async fn filter(&self, exchange: &mut dyn ServerWebExchange) -> Result<(), GatewayError> {
        let wrapper = DefaultGatewayFilterChainWrapper {
            chain: self,
            index: 0,
        };
        wrapper.filter(exchange).await
    }
}

/// Carries the position in the chain through the recursive calls.
struct DefaultGatewayFilterChainWrapper<'a> {
    chain: &'a DefaultGatewayFilterChain,
    index: usize,
}

#[async_trait]
impl GatewayFilterChain for DefaultGatewayFilterChainWrapper<'_> {
    async fn filter(&self, exchange: &mut dyn ServerWebExchange) -> Result<(), GatewayError> {
        let Some(filter) = self.chain.filters.get(self.index) else {
            return Ok(());
        };

        let chain = DefaultGatewayFilterChainWrapper {
            chain: self.chain,
            index: self.index + 1,
        };

        filter.filter(exchange, &chain).await
    }
}
