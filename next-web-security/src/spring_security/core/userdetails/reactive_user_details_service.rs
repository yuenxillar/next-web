use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::{
    user_details::UserDetails, username_not_found_error::UsernameNotFoundError,
};

/// Reactive API for finding UserDetails by username.
#[async_trait]
pub trait ReactiveUserDetailsService: Send + Sync {
    async fn find_by_username(
        &self,
        username: String,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError>;
}
