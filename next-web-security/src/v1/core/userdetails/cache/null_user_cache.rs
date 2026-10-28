use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::{UserCache, UserDetails};

/// Does not perform any caching.
#[derive(Default)]
pub struct NullUserCache;

#[allow(unused_variables)]
#[async_trait]
impl UserCache for NullUserCache {
    async fn get_user_from_cache(&self, username: &str) -> Option<Arc<dyn UserDetails>> {
        None
    }

    async fn put_user_in_cache(&self, user: Arc<dyn UserDetails>) {}

    async fn remove_user_from_cache(&self, username: &str) {}
}
