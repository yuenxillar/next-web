use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    core::{
        userdetails::{UserDetails, UserDetailsService},
        AuthenticationError,
    },
    provisioning::UserDetailsManager,
};

pub struct InMemoryUserDetailsManager;

impl InMemoryUserDetailsManager {
    pub fn new() -> Self {
        Self {}
    }

    pub fn with_users(users: Vec<Arc<dyn UserDetails>>) -> Self {
        Self {}
    }
}

impl UserDetailsManager for InMemoryUserDetailsManager {}

#[async_trait]
impl UserDetailsService for InMemoryUserDetailsManager {
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError> {
        todo!()
    }
}
