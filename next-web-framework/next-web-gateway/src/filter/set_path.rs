use crate::{filter::gateway_filter::GatewayFilter, route::route_service_manager::UpStream};


#[derive(Debug, Clone)]
pub struct SetPathFilter {
    pub path: String,
}

impl GatewayFilter  for SetPathFilter {
    fn filter(
        &self,
        _ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        upstream: &mut UpStream,
    ) {
    //    let path = request_header.uri.path();
    //    if path.is
    }
}