use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct SetRequestHostHeaderFilter {
    pub host: String,
}

impl DefaultGatewayFilter for SetRequestHostHeaderFilter {
    fn filter(
        &self,
        session: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
    }
}
