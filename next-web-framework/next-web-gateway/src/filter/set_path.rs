use crate::filter::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct SetPathFilter {
    pub path: String,
}

impl GatewayFilter  for SetPathFilter {
    fn filter(
        &self,
        _ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
       let path = request_header.uri.path();
    //    if path.is
    }
}