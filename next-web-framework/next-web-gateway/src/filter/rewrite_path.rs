use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RewritePathFilter {}

impl GatewayFilter for RewritePathFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
    }
}
