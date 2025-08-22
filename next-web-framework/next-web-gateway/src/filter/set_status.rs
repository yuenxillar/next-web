use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SetStatusFilter {
    pub status: u16,
}

impl GatewayFilter for SetStatusFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        _request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        respnose_header.set_status(self.status).ok();
    }
}
