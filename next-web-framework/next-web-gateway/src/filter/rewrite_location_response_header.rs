use regex::Regex;

use super::gateway_filter::GatewayFilter;


#[derive(Debug, Clone)]
pub struct RewriteLocationResponseHeaderFilter {
    pub strip_version_mode:Box<str>,
    pub location_header_name: Option<Box<str>>,
    pub host_value: Option<Box<str>>,
    pub protocols_regex: Option<Regex>
}

impl GatewayFilter for RewriteLocationResponseHeaderFilter {
    fn filter(
        &self,
        ctx: &mut crate::application::next_gateway_application::ApplicationContext,
        request_header: &mut pingora::prelude::RequestHeader,
        respnose_header: &mut pingora::http::ResponseHeader,
    ) {
        todo!()
    }
}