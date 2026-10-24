use std::sync::Arc;

use next_web_core::http::{filter::HttpFilter, HttpRequest};

pub trait SecurityFilterChain: Send + Sync {
    fn matches(&self, request: &mut dyn HttpRequest) -> bool;

    fn get_filters(&self) -> &[Arc<dyn HttpFilter>];
}
