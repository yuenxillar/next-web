use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;
use crate::util::path::{set_request_path, strip_prefix_segments};

#[derive(Debug, Clone)]
pub struct StripPrefixFilter {
    pub offset: usize,
}

impl GatewayFilter for StripPrefixFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        let query = request_header.uri.query().map(str::to_owned);

        // Drop the configured number of leading path segments and preserve the query string.
        let rewritten_path = strip_prefix_segments(request_header.uri.path(), self.offset);
        set_request_path(request_header, &rewritten_path, query.as_deref());
        Ok(())
    }
}
