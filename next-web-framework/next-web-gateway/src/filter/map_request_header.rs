use crate::{route::route_service_manager::UpStream, util::key_value::KeyValue};

use super::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct MapRequestHeaderFilter{
    pub header: KeyValue<Box<str>>
}

impl GatewayFilter for MapRequestHeaderFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        upstream: &mut UpStream,
    ) {
        todo!()
    }
}