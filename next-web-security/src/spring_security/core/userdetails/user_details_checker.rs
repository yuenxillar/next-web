use next_web_core::async_trait;

use crate::core::{
    authentication_error::AuthenticationError, userdetails::user_details::UserDetails,
};

#[async_trait]
pub trait UserDetailsChecker: Send + Sync {
    async fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError>;
}
