use crate::{filter::gateway_filter::GatewayFilter, route::route_service_manager::UpStream};

#[derive(Debug, Clone)]
pub struct SecureHeadersFilter;

impl GatewayFilter for SecureHeadersFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        upstream: &mut UpStream,
    ) {
        todo!()
    }
}
