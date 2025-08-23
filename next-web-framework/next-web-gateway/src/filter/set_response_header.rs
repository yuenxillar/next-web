use super::gateway_filter::GatewayFilter;
use crate::util::key_value::KeyValue;
use crate::application::next_gateway_application::ApplicationContext;

#[derive(Debug, Clone)]
pub struct SetResponseHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for SetResponseHeaderFilter {
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
