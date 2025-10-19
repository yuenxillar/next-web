use axum::extract::Request;

use crate::core::filter::Filter;

pub trait SecurityFilterChain: Send + Sync {
    fn matches(&self, request: &Request) -> bool;

    fn get_filters(&self) -> Vec<&dyn Filter>;
}