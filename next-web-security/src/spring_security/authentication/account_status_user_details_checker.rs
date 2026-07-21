use next_web_core::async_trait;

use crate::{
    authentication::account_status_user_details_exceptions::{
        account_expired, credentials_expired, disabled, locked,
    },
    core::{
        authentication_error::AuthenticationError,
        userdetails::{UserDetails, UserDetailsChecker},
    },
};

#[derive(Clone, Default)]
pub struct AccountStatusUserDetailsChecker;

#[async_trait]
impl UserDetailsChecker for AccountStatusUserDetailsChecker {
    fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError> {
        if !to_check.is_account_non_locked() {
            return Err(locked());
        }
        if !to_check.is_enabled() {
            return Err(disabled());
        }
        if !to_check.is_account_non_expired() {
            return Err(account_expired());
        }
        if !to_check.is_credentials_non_expired() {
            return Err(credentials_expired());
        }
        Ok(())
    }
}
