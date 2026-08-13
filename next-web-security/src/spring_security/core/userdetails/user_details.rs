use std::{any::Any, sync::Arc};

use crate::core::GrantedAuthority;

pub trait UserDetails
where
    Self: Send + Sync,
    Self: Any,
{
    /// Returns the authorities granted to the user.
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>];

    /// Returns the password used to authenticate the user.
    fn password(&self) -> Option<&str>;

    /// Returns the username used to authenticate the user.
    fn username(&self) -> &str;

    /// Indicates whether the user's account has expired. An expired account cannot be authenticated.
    fn is_account_non_expired(&self) -> bool {
        true
    }

    /// Indicates whether the user is locked or unlocked. A locked user cannot be authenticated.
    fn is_account_non_locked(&self) -> bool {
        true
    }

    /// Indicates whether the user's credentials (password) has expired. Expired credentials prevent authentication.
    fn is_credentials_non_expired(&self) -> bool {
        true
    }

    /// Indicates whether the user is enabled or disabled. A disabled user cannot be authenticated.
    fn is_enabled(&self) -> bool {
        true
    }
}
