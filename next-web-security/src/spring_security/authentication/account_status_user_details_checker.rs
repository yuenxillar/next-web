use next_web_core::async_trait;

use crate::{
    authentication::account_status_user_details_exceptions::{
        account_expired, credentials_expired, disabled, locked,
    },
    core::{
        authentication_error::AuthenticationError,
        userdetails::{user_details::UserDetails, user_details_checker::UserDetailsChecker},
    },
};

#[derive(Clone, Default)]
pub struct AccountStatusUserDetailsChecker;

#[async_trait]
impl UserDetailsChecker for AccountStatusUserDetailsChecker {
    async fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError> {
        if !to_check.is_account_non_locked().await {
            return Err(locked());
        }
        if !to_check.is_enabled().await {
            return Err(disabled());
        }
        if !to_check.is_account_non_expired().await {
            return Err(account_expired());
        }
        if !to_check.is_credentials_non_expired().await {
            return Err(credentials_expired());
        }
        Ok(())
    }
}
