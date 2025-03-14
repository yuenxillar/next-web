use crate::application::next_gateway_application::ApplicationContext;

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct SaveSessionFilter {}

impl DefaultGatewayFilter for SaveSessionFilter {
    fn filter(
        &self,
        ctx: &mut ApplicationContext,
        request_header: &mut pingora::http::RequestHeader,
        _respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        if let Some(session) = &ctx.session {
            request_header
                .insert_header(
                    "Set-Cookie".to_string(),
                    format!("session_id={}; Path=/", session).as_str(),
                )
                .ok();
        }
    }
}
