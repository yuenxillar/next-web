use super::gateway_filter::DefaultGatewayFilter;
use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

#[derive(Debug, Clone)]
pub struct SetResponseHeaderFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for SetResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        _request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        for header in self.headers.iter() {
            respnose_header
                .insert_header(header.k.clone(), &header.v)
                .ok();
        }
    }
}
