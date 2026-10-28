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
        if let Some(cache) = &self.user_cache {
            if let Some(user) = cache.get_user_from_cache(username).await {
                return Ok(user);
            }
        }
        let user = self.delegate.load_user_by_username(username).await?;
        if let Some(cache) = &self.user_cache {
            cache.put_user_in_cache(user.clone()).await;
        }
        Ok(user)
    }
}
