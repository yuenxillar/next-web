use regex::Regex;

use crate::application::{key_value::KeyValue, next_gateway_application::ApplicationContext};

use super::gateway_filter::DefaultGatewayFilter;

#[derive(Debug, Clone)]
pub struct RewriteResponseHeaderFilter {
    pub header: (KeyValue<String>, Option<Regex>),
}

impl DefaultGatewayFilter for RewriteResponseHeaderFilter {
    fn filter(
        &self,
        _ctx: &mut ApplicationContext,
        _request_header: &mut pingora::http::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        if let Some(value) = respnose_header.headers.get(&self.header.0.k) {
            if let Some(regex) = &self.header.1 {
                if regex.is_match(value.to_str().unwrap_or_default()) {
                    respnose_header
                        .insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
                }
            } else {
                respnose_header.insert_header(self.header.0.k.clone(), self.header.0.v.as_str());
            }
        }
    }
}
