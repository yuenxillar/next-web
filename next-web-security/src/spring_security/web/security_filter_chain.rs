use std::sync::Arc;

use next_web_core::traits::{filter::HttpFilter, http::http_request::HttpRequest};

pub trait SecurityFilterChain: Send + Sync {
    fn matches(&self, request: &mut dyn HttpRequest) -> bool;

    fn get_filters(&self) -> &[Arc<dyn HttpFilter>];
}
