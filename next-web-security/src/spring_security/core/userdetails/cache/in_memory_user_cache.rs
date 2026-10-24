use std::{collections::HashMap, sync::Arc};

use next_web_core::async_trait;
use tokio::sync::RwLock;

use crate::core::userdetails::{UserCache, UserDetails};

#[derive(Default)]
pub struct InMemoryUserCache {
    users: RwLock<HashMap<String, Arc<dyn UserDetails>>>,
}

#[async_trait]
impl UserCache for InMemoryUserCache {
    async fn get_user_from_cache(&self, username: &str) -> Option<Arc<dyn UserDetails>> {
        self.users.read().await.get(username).cloned()
    }

    async fn put_user_in_cache(&self, user: Arc<dyn UserDetails>) {
        self.users
            .write()
            .await
            .insert(user.username().to_string(), user);
    }

    async fn remove_user_from_cache(&self, username: &str) {
        self.users.write().await.remove(username);
    }
}
