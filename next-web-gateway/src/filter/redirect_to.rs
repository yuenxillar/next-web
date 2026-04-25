use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RedirectToFilter {
    pub status: u16,
    pub url: Box<str>,
}

impl GatewayFilter for RedirectToFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        _upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        // Short-circuit the proxy flow and send a redirect directly to the downstream client.
        ctx.respond_with_empty(
            self.status,
            vec![("Location".to_string(), self.url.to_string())],
        )
    }
}
