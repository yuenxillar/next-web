use std::sync::Arc;

use next_web_core::async_trait;

use crate::core::{userdetails::UserDetails, Authentication, AuthenticationError};

/// Interface that allows for retrieving a UserDetails object based on an Authentication object.
#[async_trait]
pub trait AuthenticationUserDetailsService<T>
where
    Self: Send + Sync,
    T: Authentication,
{
    /// Retrieves user details for the given pre-authenticated authentication token.
    ///
    /// # Parameters
    /// * `token` - The pre-authenticated authentication token
    ///
    /// # Returns
    /// UserDetails for the given authentication token. This method never returns `none`.
    ///
    /// # Errors
    /// Returns [`AuthenticationError`] if no user details can be found for the given
    /// authentication token.
    async fn load_user_details(
        &self,
        token: &T,
    ) -> Result<Arc<dyn UserDetails>, AuthenticationError>;
}
