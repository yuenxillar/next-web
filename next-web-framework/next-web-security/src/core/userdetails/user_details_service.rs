use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::userdetails::user_details::UserDetails;

#[async_trait]
pub trait UserDetailsService
where
    Self: Send + Sync,
{
    async fn load_user_by_username(
        &self,
        username: String,
    ) -> Result<Arc<dyn UserDetails>, UsernameNotFoundError>;
}
