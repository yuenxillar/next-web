use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    user_cache::{NullUserCache, UserCache},
    userdetails::{
        username_not_found_error::UsernameNotFoundError, UserDetails, UserDetailsService,
    },
};

pub struct CachingUserDetailsService {
    user_cache: Arc<dyn UserCache>,
    delegate: Arc<dyn UserDetailsService>,
}

impl CachingUserDetailsService {
    pub fn new(delegate: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_cache: Arc::new(NullUserCache),
            delegate,
        }
    }

    pub fn user_cache(&self) -> Arc<dyn UserCache> {
        self.user_cache.clone()
    }

    pub fn set_user_cache(&mut self, user_cache: Arc<dyn UserCache>) {
        self.user_cache = user_cache;
    }
}

#[async_trait]
impl UserDetailsService for CachingUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: String,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        if let Some(user) = self.user_cache.get_user_from_cache(&username) {
            return Ok(user);
        }

        let user = self
            .delegate
            .load_user_by_username(username.clone())
            .await?;
        let cache_key = user.username();
        self.user_cache
            .put_user_in_cache(cache_key.to_string(), user.clone());
        Ok(user)
    }
}
