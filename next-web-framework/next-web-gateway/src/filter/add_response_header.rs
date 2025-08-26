use crate::application::next_gateway_application::ApplicationContext;
use crate::route::route_service_manager::UpStream;
use crate::util::key_value::KeyValue;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct AddResponseHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for AddResponseHeaderFilter {
    fn filter(&self, _ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return,
        };

        self.headers.iter().for_each(|header| {
            response_header
                .append_header(header.k.clone(), header.v.as_str())
                .ok();
        });
    }
}
