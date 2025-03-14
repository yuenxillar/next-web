use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct SetRequestHostHeaderFilter {
    pub host: String,
}

impl DefaultGatewayFilter for SetRequestHostHeaderFilter {
    fn filter(
        &self,
        _session: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        request_header
            .insert_header("host".to_string(), self.host.as_str())
            .ok();
    }
}
