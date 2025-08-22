use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RequestSizeFilter {
    // byte
    pub max_size: u64,
}

impl GatewayFilter for RequestSizeFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _response_header: &mut pingora::http::ResponseHeader,
    ) {
        if let Some(content_length) = request_header.headers.get("content-length") {
            if let Ok(content_length) = content_length.to_str() {
                if let Ok(size) = content_length.parse::<u64>() {
                    if size > self.max_size {
                        return;
                    }
                }
            }
        }
    }
}
