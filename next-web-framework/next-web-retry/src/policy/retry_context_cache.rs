use std::sync::Arc;

use crate::retry_context::RetryContext;

pub trait RetryContextCache
where
    Self: Send + Sync
{
    fn get(&self, key: &str) -> Option<&dyn  RetryContext>;
    
    fn put(&mut self, key: &str, value: Arc<dyn RetryContext>);

    fn remove(&mut self, key: &str);

    fn contains_key(&self, key: &str) -> bool;
}
