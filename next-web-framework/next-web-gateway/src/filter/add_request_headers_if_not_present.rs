use super::gateway_filter::DefaultGatewayFilter;
use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

#[derive(Debug, Clone)]
pub struct AddRequestHeaderIfNotPresentFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for AddRequestHeaderIfNotPresentFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        // if let Some(request_header) = request_header {
        //     for header in self.headers.iter() {
        //         request_header.append_header(header.k.clone(), &header.v);
        //     }
        // }
    }
}
