use regex::Regex;

use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;
use crate::util::path::set_request_path;

#[derive(Debug, Clone)]
pub struct RewritePathFilter {
    pub regex: Regex,
    pub replacement: String,
}

impl GatewayFilter for RewritePathFilter {
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

        // Rewrite only the path component and keep the original query string untouched.
        let rewritten_path = self
            .regex
            .replace(request_header.uri.path(), self.replacement.as_str())
            .to_string();

        set_request_path(request_header, &rewritten_path, query.as_deref());
        Ok(())
    }
}
