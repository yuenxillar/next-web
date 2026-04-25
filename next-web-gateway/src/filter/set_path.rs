use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;
use crate::util::path::set_request_path;

#[derive(Debug, Clone)]
pub struct SetPathFilter {
    pub path: String,
}

impl GatewayFilter for SetPathFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        let original_query = request_header.uri.query().map(str::to_owned);
        let (path, query) = match self.path.split_once('?') {
            Some((path, query)) => (path.to_string(), Some(query.to_string())),
            None => (self.path.clone(), original_query),
        };

        // Replace the path while preserving the original query unless a new one is configured.
        set_request_path(request_header, &path, query.as_deref());
        Ok(())
    }
}
