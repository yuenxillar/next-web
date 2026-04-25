use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RequestHeaderSizeFilter {
    pub max_size: u32,
    pub error_message: String,
}

impl GatewayFilter for RequestHeaderSizeFilter {
    fn filter(&self, ctx: &mut ApplicationContext, upstream: &mut UpStream) -> pingora::Result<()> {
        if self.max_size == 0 {
            return Ok(());
        }

        let request_header = match upstream.request_header.as_ref() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        // Estimate the serialized header size so oversized requests can be rejected early.
        let header_size = request_header
            .headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.as_bytes().len() + 4)
            .sum::<usize>()
            + 2;

        if header_size > self.max_size as usize {
            return ctx.respond_with_text(431, Vec::new(), self.error_message.clone());
        }

        Ok(())
    }
}
