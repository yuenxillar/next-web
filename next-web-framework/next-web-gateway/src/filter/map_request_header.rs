use crate::util::key_value::KeyValue;

use super::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct MapRequestHeaderFilter{
    pub header: KeyValue<Box<str>>
}

impl GatewayFilter for MapRequestHeaderFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        todo!()
    }
}