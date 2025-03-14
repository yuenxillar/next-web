use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct RemoveRequestHeaderFilter {
    pub headers: Vec<String>,
}

impl DefaultGatewayFilter for RemoveRequestHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _response_header: &mut pingora::http::ResponseHeader,
    ) {
        // Traverse the list of headers that need to be removed
        for header_name in &self.headers {
            let header_name = header_name.to_lowercase();
            request_header.remove_header(&header_name);
        }
    }
}
