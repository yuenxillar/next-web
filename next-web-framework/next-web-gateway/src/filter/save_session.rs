use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct SaveSessionFilter {}

impl DefaultGatewayFilter for SaveSessionFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
    }
}
