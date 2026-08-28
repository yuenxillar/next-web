use std::{any::Any, sync::Arc};

use crate::core::GrantedAuthority;

/// Provides core user information.
/// Implementations are not used directly by Spring Security for security purposes. They simply store user
/// information which is later encapsulated into Authentication objects. This allows non-security related
/// user information (such as email addresses, telephone numbers etc) to be stored in a convenient location.
///
/// Concrete implementations must take particular care to ensure the non-null contract detailed for each method is enforced.
/// See User for a reference implementation (which you might like to extend or use in your code).
pub trait UserDetails
where
    Self: Send + Sync,
    Self: Any,
{
    /// Returns the authorities granted to the user.
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>];

    /// Returns the password used to authenticate the user.
    /// Can be none if the user has not specified a password (e.g. the user Passkeys instead).
    fn password(&self) -> Option<&str>;

    /// Returns the username used to authenticate the user. Cannot return none.
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
