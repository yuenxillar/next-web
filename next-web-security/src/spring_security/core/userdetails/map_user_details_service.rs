use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use next_web_core::async_trait;

use crate::core::{
    authority_utils::AuthorityUtils,
    userdetails::{
        user::User, user_details::UserDetails,
        user_details_password_service::UserDetailsPasswordService,
        user_details_service::UserDetailsService, username_not_found_error::UsernameNotFoundError,
    },
};

#[derive(Default)]
pub struct MapUserDetailsService {
    users: RwLock<HashMap<String, Arc<dyn UserDetails>>>,
}

impl MapUserDetailsService {
    pub fn new(users: impl IntoIterator<Item = Arc<dyn UserDetails>>) -> Self {
        let service = Self::default();
        for user in users {
            service.create_user(user);
        }
        service
    }

    pub fn create_user(&self, user: Arc<dyn UserDetails>) {
        let key = user.username().to_lowercase();
        if let Ok(mut users) = self.users.write() {
            users.insert(key, user);
        }
    }

    pub fn user_exists(&self, username: &str) -> bool {
        self.users
            .read()
            .map(|users| users.contains_key(&Self::key(username)))
            .unwrap_or(false)
    }

    pub fn delete_user(&self, username: &str) {
        if let Ok(mut users) = self.users.write() {
            users.remove(&Self::key(username));
        }
    }

    fn key(username: &str) -> String {
        username.to_lowercase()
    }

    async fn with_new_password(
        user: Arc<dyn UserDetails>,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        let authorities = user
            .authorities()
            .into_iter()
            .filter_map(|authority| authority.authority())
            .collect::<Vec<_>>();

        Arc::new(User::with_flags(
            user.username(),
            new_password,
            user.is_enabled(),
            user.is_account_non_expired(),
            user.is_credentials_non_expired(),
            user.is_account_non_locked(),
            AuthorityUtils::create_authority_list(authorities),
        ))
    }
}

#[async_trait]
impl UserDetailsService for MapUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        self.users
            .read()
            .ok()
            .and_then(|users| users.get(&Self::key(&username)).cloned())
            .ok_or(UsernameNotFoundError(username.to_owned()))
    }
}

#[async_trait]
impl UserDetailsPasswordService for MapUserDetailsService {
    async fn update_password(
        &self,
        user: Arc<dyn UserDetails>,
        new_password: Option<String>,
    ) -> Arc<dyn UserDetails> {
        let updated = Self::with_new_password(user.clone(), new_password).await;
        let key = user.username().to_lowercase();
        if let Ok(mut users) = self.users.write() {
            users.insert(key, updated.clone());
        }
        updated
    }
}
