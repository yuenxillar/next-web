use next_web_core::error::BoxError;

use crate::web::authentication::rememberme::PersistentRememberMeToken;

/// The abstraction used by PersistentTokenBasedRememberMeServices to store the persistent login tokens for a user.
pub trait PersistentTokenRepository
where
    Self: Send + Sync,
{
    fn create_new_token(&self, token: &PersistentRememberMeToken) -> Result<(), BoxError>;

    fn update_token(&self, series: &str, token_value: &str, last_used: i64)
        -> Result<(), BoxError>;

    fn get_token_for_series(&self, series_id: &str) -> Option<&PersistentRememberMeToken>;

    fn remove_user_tokens(&self, username: &str);
}
