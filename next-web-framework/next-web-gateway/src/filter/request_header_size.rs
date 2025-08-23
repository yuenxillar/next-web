use super::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct RequestHeaderSizeFilter {
    pub max_size: u32,
    pub error_message: String,
}


impl GatewayFilter for RequestHeaderSizeFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        todo!()
    }
}