use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{authentication::Authentication, userdetails::user_details::UserDetails};

use super::username_not_found_error::UsernameNotFoundError;

#[async_trait]
pub trait AuthenticationUserDetailsService<T>: Send + Sync
where
    T: Authentication + Send + Sync,
{
    async fn load_user_details(
        &self,
        token: &T,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError>;
}
