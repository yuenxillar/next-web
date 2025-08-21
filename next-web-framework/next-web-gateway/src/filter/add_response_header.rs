use crate::util::key_value::KeyValue;
use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct AddResponseHeaderFilter {
    pub headers: Vec<KeyValue<String>>,
}

impl DefaultGatewayFilter for AddResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        _request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        for header in &self.headers {
            respnose_header
                .append_header(header.k.clone(), header.v.as_str())
                .ok();
        }
    }
}
