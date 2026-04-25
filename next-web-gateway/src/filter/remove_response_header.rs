use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RemoveResponseHeaderFilter {
    pub headers: Vec<String>,
}

impl GatewayFilter for RemoveResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return Ok(()),
        };

        // Remove every configured header name from the proxied response.
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            response_header.remove_header(&header_name);
        }

        Ok(())
    }
}
