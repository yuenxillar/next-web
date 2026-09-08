use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    userdetails::{UserCache, UserDetails, UserDetailsService},
    AuthenticationError,
};

pub struct CachingUserDetailsService {
    user_cache: Option<Arc<dyn UserCache>>,
    delegate: Arc<dyn UserDetailsService>,
}

impl CachingUserDetailsService {
    pub fn new(delegate: Arc<dyn UserDetailsService>) -> Self {
        Self {
            user_cache: None,
            delegate,
        }
    }

    pub fn user_cache(&self) -> Option<&Arc<dyn UserCache>> {
        self.user_cache.as_ref()
    }

    pub fn set_user_cache(&mut self, user_cache: Arc<dyn UserCache>) {
        self.user_cache = Some(user_cache);
    }
}

#[async_trait]
impl UserDetailsService for CachingUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        // match self.user_cache.as_ref() {
        //     Some(user_cache) => user_cache,
        //     None => return,
        // }

        // if let Some(user) = self.user_cache.get_user_from_cache(username).await {
        //     return Ok(user);
        // }

        // let cache_key = self.delegate.user;
        // self.user_cache
        //     .put_user_in_cache(cache_key.to_string(), self.delegate.clone());
        // Ok(user)
        todo!()
    }
}
