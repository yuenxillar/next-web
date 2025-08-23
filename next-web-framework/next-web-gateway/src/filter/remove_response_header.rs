use super::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct RemoveResponseHeaderFilter {
    pub headers: Vec<String>,
}

impl GatewayFilter for RemoveResponseHeaderFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        todo!()
    }
}