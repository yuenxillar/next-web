use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct PreserveHostHeaderFilter {}

impl GatewayFilter for PreserveHostHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        if request_header.headers.contains_key("host") {
            return Ok(());
        }

        let authority = request_header
            .uri
            .authority()
            .map(|authority| authority.as_str().to_string());

        if let Some(authority) = authority {
            // Restore the downstream host header when only the URI authority is available.
            request_header.insert_header("Host", authority).ok();
        }

        Ok(())
    }
}
