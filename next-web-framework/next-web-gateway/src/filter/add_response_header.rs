use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct AddResponseHeaderFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for AddResponseHeaderFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
    }
}
