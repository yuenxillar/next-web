use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RemoveRequestHeaderFilter {
    pub headers: Vec<String>,
}

impl GatewayFilter for RemoveRequestHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        // Remove every configured header name from the proxied request.
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            request_header.remove_header(&header_name);
        }

        Ok(())
    }
}
