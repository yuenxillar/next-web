use pingora::http::StatusCode;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RedirectToFilter {
    pub status: u16,
    pub url: Box<str>
}

impl GatewayFilter for RedirectToFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        todo!()
    }
}