use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream, util::key_value::KeyValue,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct MapRequestHeaderFilter {
    pub header: KeyValue<Box<str>>,
}

impl GatewayFilter for MapRequestHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        upstream: &mut UpStream,
    ) -> pingora::Result<()> {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return Ok(()),
        };

        let source = self.header.k.as_ref();
        let target = self.header.v.to_string();

        if source.eq_ignore_ascii_case(&target) {
            return Ok(());
        }

        // Copy every source header value onto the target header without removing the source.
        let values = request_header
            .headers
            .get_all(source)
            .iter()
            .filter_map(|value| value.to_str().ok().map(str::to_owned))
            .collect::<Vec<_>>();

        for value in values {
            request_header.append_header(target.clone(), value).ok();
        }

        Ok(())
    }
}
