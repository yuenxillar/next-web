use super::gateway_filter::DefaultGatewayFilter;
use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

#[derive(Debug, Clone)]
pub struct AddRequestHeaderFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for AddRequestHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        for header in self.headers.iter() {
            request_header
                .append_header(header.k.clone(), &header.v)
                .ok();
        }
    }
}
