use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{
    user_cache::{NullUserCache, UserCache},
    userdetails::{
        user_details::UserDetails, user_details_service::UserDetailsService,
        username_not_found_error::UsernameNotFoundError,
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

        let user = self.delegate.load_user_by_username(username.clone()).await?;
        let cache_key = user.get_username().await;
        self.user_cache.put_user_in_cache(cache_key, user.clone());
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
    };

    use next_web_core::async_trait;

    use crate::{
        authentication::caching_user_details_service::CachingUserDetailsService,
        core::{
            authority_utils::AuthorityUtils,
            user_cache::InMemoryUserCache,
            userdetails::{
                user::User,
                user_details::UserDetails,
                user_details_service::UserDetailsService,
                username_not_found_error::UsernameNotFoundError,
            },
        },
    };

    struct CountingUserDetailsService {
        loads: AtomicUsize,
    }

    #[async_trait]
    impl UserDetailsService for CountingUserDetailsService {
        async fn load_user_by_username(
            &self,
            username: String,
        ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
            self.loads.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(User::new(
                username,
                Some(String::from("password")),
                AuthorityUtils::create_authority_list(["ROLE_USER"]),
            )))
        }
    }

    #[tokio::test]
    async fn caching_service_delegates_once_then_uses_cache() {
        let delegate = Arc::new(CountingUserDetailsService {
            loads: AtomicUsize::new(0),
        });
        let mut service = CachingUserDetailsService::new(delegate.clone());
        service.set_user_cache(Arc::new(InMemoryUserCache::default()));

        let first = service
            .load_user_by_username(String::from("alice"))
            .await
            .unwrap();
        let second = service
            .load_user_by_username(String::from("alice"))
            .await
            .unwrap();

        assert_eq!(first.get_username().await, "alice");
        assert_eq!(second.get_username().await, "alice");
        assert_eq!(delegate.loads.load(Ordering::SeqCst), 1);
    }
}
