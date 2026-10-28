use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{userdetails::UserDetails, AuthenticationError};

/// Core interface which loads user-specific data.
/// It is used throughout the framework as a user DAO and is the strategy used by the DaoAuthenticationProvider.
/// The interface requires only one read-only method, which simplifies support for new data-access strategies.
#[async_trait]
pub trait UserDetailsService
where
    Self: Send + Sync,
{
    /// Locates the user based on the username. In the actual implementation, the search may possibly be
    /// case sensitive, or case insensitive depending on how the implementation instance is configured.
    /// In this case, the UserDetails object that comes back may have a username that is of a different
    /// case than what was actually requested.
    async fn load_user_by_username(
        &self,
        username: &str,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError>;
}
