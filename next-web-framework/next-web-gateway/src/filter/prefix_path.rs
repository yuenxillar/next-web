use crate::route::route_service_manager::UpStream;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct PrefixPathFilter {
    pub path: Box<str>
}

impl GatewayFilter for PrefixPathFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        upstream: &mut UpStream,
    ) {
        todo!()
    }
}