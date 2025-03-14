use super::gateway_filter::DefaultGatewayFilter;
use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

#[derive(Debug, Clone)]
pub struct AddRequestHeaderIfNotPresentFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for AddRequestHeaderIfNotPresentFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        for header in &self.headers {
            if !request_header.headers.contains_key(header.k.as_str()) {
                request_header
                    .insert_header(header.k.clone(), header.v.as_str())
                    .ok();
            }
        }
    }
}
