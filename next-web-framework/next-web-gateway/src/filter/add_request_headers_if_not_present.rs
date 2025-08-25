use super::gateway_filter::GatewayFilter;
use crate::application::next_gateway_application::ApplicationContext;
use crate::route::route_service_manager::UpStream;
use crate::util::key_value::KeyValue;

#[derive(Debug, Clone)]
pub struct AddRequestHeaderIfNotPresentFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for AddRequestHeaderIfNotPresentFilter {
    fn filter(&self, _ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        upstream.request_header.as_mut().map(|request_header| {
            for header in &self.headers {
                if !request_header.headers.contains_key(header.k.as_str()) {
                    request_header
                        .insert_header(header.k.clone(), header.v.as_str())
                        .ok();
                }
            }
        });
    }
}
