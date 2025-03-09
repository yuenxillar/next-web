use super::gateway_filter::DefaultGatewayFilter;
use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

#[derive(Debug, Clone)]
pub struct SetResponseHeaderFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for SetResponseHeaderFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
    }
}
