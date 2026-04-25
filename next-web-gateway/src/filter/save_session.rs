use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SaveSessionFilter {}

impl GatewayFilter for SaveSessionFilter {
    fn filter(&self, ctx: &mut ApplicationContext, upstream: &mut UpStream) -> pingora::Result<()> {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return Ok(()),
        };

        if let Some(session) = &ctx.session {
            // Persist the gateway session as a response cookie when one exists on the context.
            response_header
                .append_header(
                    "Set-Cookie",
                    format!("session_id={session}; Path=/; HttpOnly; SameSite=Lax"),
                )
                .ok();
        }

        Ok(())
    }
}
