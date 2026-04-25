use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SetRequestHostHeaderFilter {
    pub host: String,
}

impl GatewayFilter for SetRequestHostHeaderFilter {
    fn filter(
        &self,
        _session: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        upstream.request_header.as_mut().map(|request_header| {
            // Set the upstream Host header explicitly so H1 and H2 forwarding use the same host.
            request_header
                .insert_header("Host".to_string(), self.host.as_str())
                .ok();
        });

        Ok(())
    }
}
