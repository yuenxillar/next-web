use super::gateway_filter::GatewayFilter;
use crate::util::key_value::KeyValue;
use crate::application::next_gateway_application::ApplicationContext;

#[derive(Debug, Clone)]
pub struct AddRequestHeaderIfNotPresentFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for AddRequestHeaderIfNotPresentFilter {
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
