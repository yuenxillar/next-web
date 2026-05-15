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

#[cfg(test)]
mod tests {
    use crate::{
        core::{
            authority_utils::AuthorityUtils,
            userdetails::{user::User, user_details_checker::UserDetailsChecker},
        },
    };

    use super::AccountStatusUserDetailsChecker;

    #[tokio::test]
    async fn account_status_checker_rejects_locked_users() {
        let checker = AccountStatusUserDetailsChecker;
        let user = User::with_flags(
            "alice",
            Some(String::from("secret")),
            true,
            true,
            true,
            false,
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );

        let error = checker.check(&user).await.unwrap_err();
        assert_eq!(error.get_message(), "User account is locked");
    }
}
