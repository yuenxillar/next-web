use super::gateway_filter::GatewayFilter;
use crate::application::next_gateway_application::ApplicationContext;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct AddRequestHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for AddRequestHeaderFilter {
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
