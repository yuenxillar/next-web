use std::sync::Arc;

use crate::{policy::retry_context_cache::RetryContextCache, retry_context::RetryContext};

#[derive(Clone)]
pub struct MapRetryContextCache {}

impl MapRetryContextCache {
    pub fn new() -> Self {
        Self {}
    }
}

impl RetryContextCache for MapRetryContextCache {
    fn get(&self, key: &str) -> Option<&dyn crate::retry_context::RetryContext> {
        todo!()
    }

    fn get_mut(&self, key: &str) -> Option<&mut dyn RetryContext> {
        None
    }

    fn put(&mut self, key: &str, value: Arc<dyn RetryContext>) {
        todo!()
    }

    fn remove(&mut self, key: &str) {
        todo!()
    }

    fn contains_key(&self, key: &str) -> bool {
        todo!()
    }
}
