use std::{collections::HashMap, sync::Arc};

use next_web_core::async_trait;

use crate::{
    core::{
        userdetails::{UserDetails, UserDetailsService},
        {AuthenticationError, AuthenticationErrorKind},
    },
    provisioning::UserDetailsManager,
};

/// A [`UserDetailsManager`] that keeps its users in memory.
///
/// The manager is what an application uses to declare the users it knows
/// without a user store, and it is what the default user of the framework is
/// contributed with.
#[derive(Clone, Default)]
pub struct InMemoryUserDetailsManager {
    users: HashMap<String, Arc<dyn UserDetails>>,
}

impl InMemoryUserDetailsManager {
    /// Creates an empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a manager with the given users.
    ///
    /// # Arguments
    ///
    /// * `users` - The users of the manager, keyed by their username.
    pub fn with_users(users: Vec<Arc<dyn UserDetails>>) -> Self {
        Self {
            users: users
                .into_iter()
                .map(|user| (user.username().to_string(), user))
                .collect(),
        }
    }

    /// Adds a user to the manager, replacing the user with the same username.
    ///
    /// # Arguments
    ///
    /// * `user` - The user to add.
    pub fn add_user(&mut self, user: Arc<dyn UserDetails>) {
        self.users.insert(user.username().to_string(), user);
    }

    /// Removes the user with the given username.
    ///
    /// # Arguments
    ///
    /// * `username` - The username of the user.
    pub fn remove_user(&mut self, username: &str) -> Option<Arc<dyn UserDetails>> {
        self.users.remove(username)
    }
}

impl UserDetailsManager for InMemoryUserDetailsManager {}

#[async_trait]
impl UserDetailsService for InMemoryUserDetailsManager {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        self.users.get(username).cloned().ok_or_else(|| {
            AuthenticationError::with_kind(
                format!("User {username} not found"),
                AuthenticationErrorKind::UsernameNotFound,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        core::userdetails::{User, UserDetailsService},
        provisioning::InMemoryUserDetailsManager,
    };

    #[tokio::test]
    async fn a_manager_loads_the_users_it_stores() {
        let manager = InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username("user")
                .password(Some("{noop}password".to_string()))
                .roles(&["USER".to_string()])
                .build(),
        )]);

        let user = manager
            .load_user_by_username("user")
            .await
            .expect("the user is stored");

        assert_eq!(user.username(), "user");
        assert_eq!(user.password(), Some("{noop}password"));
    }

    #[tokio::test]
    async fn a_manager_reports_an_unknown_user() {
        let manager = InMemoryUserDetailsManager::default();

        let error = match manager.load_user_by_username("other").await {
            Ok(_) => panic!("the user is not stored"),
            Err(error) => error,
        };

        assert_eq!(
            error.kind(),
            crate::core::AuthenticationErrorKind::UsernameNotFound
        );
    }

    #[test]
    fn a_user_details_manager_removes_a_user() {
        let mut manager = InMemoryUserDetailsManager::default();
        manager.add_user(Arc::new(
            User::with_username("user")
                .password(Some("password".to_string()))
                .build(),
        ));

        assert!(manager.remove_user("user").is_some());
        assert!(manager.remove_user("user").is_none());
    }
}
