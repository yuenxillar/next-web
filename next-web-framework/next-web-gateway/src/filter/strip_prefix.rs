use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct StripPrefixFilter {
    pub offset: usize,
}

impl GatewayFilter for StripPrefixFilter {
    fn filter(&self, _ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        let request_header = match upstream.request_header.as_mut() {
            Some(request_header) => request_header,
            None => return,
        };

        let raw_path = String::from_utf8_lossy(request_header.raw_path());

        let mut path = String::new();
        raw_path
            .split("/")
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i >= &self.offset)
            .for_each(|(_, s)| {
                path.push('/');
                path.push_str(s);
            });

        if !path.is_empty() {
            request_header.set_uri(path.parse().unwrap());
        }
    }
}
