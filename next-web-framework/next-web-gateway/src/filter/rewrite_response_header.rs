use regex::Regex;

use crate::application::next_gateway_application::ApplicationContext;
use crate::route::route_service_manager::UpStream;
use crate::util::key_value::KeyValue;

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RewriteResponseHeaderFilter {
    pub header: (KeyValue<String, String>, Option<Regex>),
}

impl GatewayFilter for RewriteResponseHeaderFilter {
    fn filter(&self, _ctx: &mut ApplicationContext, upstream: &mut UpStream) {
        let response_header = match upstream.response_header.as_mut() {
            Some(response_header) => response_header,
            None => return,
        };

        if let Some(value) = response_header.headers.get(&self.header.0.k) {
            if let Some(regex) = &self.header.1 {
                if regex.is_match(value.to_str().unwrap_or_default()) {
                    let _ = response_header
                        .insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
                }
            } else {
                let _ = response_header
                    .insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
            }
        }
    }
}
