use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;
use crate::util::path::{join_paths, set_request_path};

#[derive(Debug, Clone)]
pub struct PrefixPathFilter {
    pub path: Box<str>,
}

impl GatewayFilter for PrefixPathFilter {
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

        // Prefix the configured path while preserving the original query string.
        let rewritten_path = join_paths(self.path.as_ref(), request_header.uri.path());
        set_request_path(request_header, &rewritten_path, query.as_deref());
        Ok(())
    }
}
