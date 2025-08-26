use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SaveSessionFilter {}

impl GatewayFilter for SaveSessionFilter {
    fn filter(&self, ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        // if let Some(session) = &ctx.session {
        //     request_header
        //         .insert_header(
        //             "Set-Cookie".to_string(),
        //             format!("session_id={}; Path=/", session).as_str(),
        //         )
        //         .ok();
        // }
    }
}
