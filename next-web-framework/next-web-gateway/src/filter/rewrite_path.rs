use regex::Regex;

use crate::{
    application::next_gateway_application::ApplicationContext,
    route::route_service_manager::UpStream,
};

use super::gateway_filter::GatewayFilter;

#[derive(Debug, Clone)]
pub struct RewritePathFilter {
    pub regex: Regex,
    pub replacement: String,
}

impl GatewayFilter for RewritePathFilter {
    fn filter(&self, ctx: &mut ApplicationContext, upstream: &mut UpStream) {}
}
