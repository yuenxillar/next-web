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
        let key = block_on(user.get_username()).to_lowercase();
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
            .get_authorities()
            .await
            .into_iter()
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect::<Vec<_>>();

        Arc::new(User::with_flags(
            user.get_username().await,
            new_password,
            user.is_enabled().await,
            user.is_account_non_expired().await,
            user.is_credentials_non_expired().await,
            user.is_account_non_locked().await,
            AuthorityUtils::create_authority_list(authorities),
        ))
    }
}

#[async_trait]
impl UserDetailsService for MapUserDetailsService {
    async fn load_user_by_username(
        &self,
        username: String,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError> {
        self.users
            .read()
            .ok()
            .and_then(|users| users.get(&Self::key(&username)).cloned())
            .ok_or(UsernameNotFoundError(username))
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
        let key = user.get_username().await.to_lowercase();
        if let Ok(mut users) = self.users.write() {
            users.insert(key, updated.clone());
        }
        updated
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    futures::executor::block_on(future)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::core::{
        authority_utils::AuthorityUtils,
        userdetails::{
            map_user_details_service::MapUserDetailsService, user::User,
            user_details_password_service::UserDetailsPasswordService,
            user_details_service::UserDetailsService,
        },
    };

    #[tokio::test]
    async fn map_service_loads_users_case_insensitively() {
        let service = MapUserDetailsService::new([Arc::new(User::new(
            "Alice",
            Some(String::from("password")),
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        )) as Arc<dyn crate::core::userdetails::user_details::UserDetails>]);

        let user = service
            .load_user_by_username(String::from("alice"))
            .await
            .unwrap();

        assert_eq!(user.get_username().await, "Alice");
        assert!(service.user_exists("ALICE"));
    }

    #[tokio::test]
    async fn map_service_updates_password_and_preserves_flags() {
        let user = Arc::new(User::with_flags(
            "bob",
            Some(String::from("old")),
            false,
            true,
            true,
            true,
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        )) as Arc<dyn crate::core::userdetails::user_details::UserDetails>;
        let service = MapUserDetailsService::new([user.clone()]);

        let updated = service
            .update_password(user, Some(String::from("new")))
            .await;

        assert_eq!(updated.get_password().await, "new");
        assert!(!updated.is_enabled().await);
        assert_eq!(
            service
                .load_user_by_username(String::from("BOB"))
                .await
                .unwrap()
                .get_password()
                .await,
            "new"
        );
    }
}
