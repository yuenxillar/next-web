use super::gateway_filter::GatewayFilter;
use crate::route::route_service_manager::UpStream;
use crate::util::key_value::KeyValue;
use crate::application::next_gateway_application::ApplicationContext;

#[derive(Debug, Clone)]
pub struct SetResponseHeaderFilter {
    pub headers: Vec<KeyValue<String, String>>,
}

impl GatewayFilter for SetResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) {
        
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return,
        };

        for header in self.headers.iter() {
            response_header
                .insert_header(header.k.clone(), &header.v)
                .ok();
        }
    }
}
