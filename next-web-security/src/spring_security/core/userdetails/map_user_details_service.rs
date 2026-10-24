use std::{collections::HashMap, sync::Arc};

use next_web_core::async_trait;
use tokio::sync::RwLock;

use crate::core::{
    userdetails::{User, UserDetails, UserDetailsPasswordService, UserDetailsService},
    AuthenticationError, AuthenticationErrorKind,
};

/// A Map based implementation of ReactiveUserDetailsService
#[derive(Default)]
pub struct MapUserDetailsService {
    users: RwLock<HashMap<String, Arc<dyn UserDetails>>>,
}

impl MapUserDetailsService {
    /// Creates a new MapUserDetailsService with the given users.
    pub fn new<I, K>(users: I) -> Self
    where
        I: IntoIterator<Item = (K, Arc<dyn UserDetails>)>,
        K: Into<String>,
    {
        Self {
            users: RwLock::new(users.into_iter().map(|(k, v)| (k.into(), v)).collect()),
        }
    }

    /// Creates a new instance
    pub fn with_users<I>(users: I) -> Self
    where
        I: IntoIterator<Item = Arc<dyn UserDetails>>,
    {
        let users = users.into_iter().collect::<Vec<_>>();
        assert!(!users.is_empty(), "users cannot be  empty");

        Self {
            users: RwLock::new(
                users
                    .into_iter()
                    .map(|user| (Self::key(user.username()), user))
                    .collect(),
            ),
        }
    }

    fn key(username: &str) -> String {
        username.to_ascii_lowercase()
    }

    fn with_new_password(
        user_details: &dyn UserDetails,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        Arc::new(
            User::with_user_details(user_details)
                .password(new_password)
                .build(),
        )
    }
}

#[async_trait]
impl UserDetailsService for MapUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        self.users
            .read()
            .await
            .get(&Self::key(username))
            .map(|user_details| {
                Arc::new(User::with_user_details(user_details.as_ref()).build())
                    as Arc<dyn UserDetails>
            })
            .ok_or(AuthenticationError::with_kind(
                username,
                AuthenticationErrorKind::UsernameNotFound,
            ))
    }
}

#[async_trait]
impl UserDetailsPasswordService for MapUserDetailsService {
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        let user_details = Self::with_new_password(user.as_ref(), new_password);
        let key = Self::key(user.username());
        self.users.write().await.insert(key, user_details.clone());

        user_details
    }
}
