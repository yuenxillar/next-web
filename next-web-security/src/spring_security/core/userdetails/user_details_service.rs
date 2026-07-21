use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::UserDetails;

use super::username_not_found_error::UsernameNotFoundError;

#[async_trait]
pub trait UserDetailsService
where
    Self: Send + Sync,
{
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError>;
}
