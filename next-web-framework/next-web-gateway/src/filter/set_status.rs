use crate::{application::next_gateway_application::ApplicationContext, route::route_service_manager::UpStream};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SetStatusFilter {
    pub status: u16,
}

impl GatewayFilter for SetStatusFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
       upstream: &mut  UpStream
    ) {
        match upstream.response_header.as_mut() {
            Some(response_header) => {
                response_header.set_status(self.status).ok();
            },
            None => return,
        }
    }
}
