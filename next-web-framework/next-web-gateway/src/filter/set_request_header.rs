use crate::application::next_gateway_application::ApplicationContext;
use crate::route::route_service_manager::UpStream;
use crate::util::key_value::KeyValue;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct SetRequestHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for SetRequestHeaderFilter {
    fn filter(&self, _ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return,
        };

        for header in self.headers.iter() {
            request_header
                .insert_header(header.k.clone(), &header.v)
                .ok();
        }
    }
}
