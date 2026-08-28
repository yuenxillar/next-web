use crate::core::{userdetails::UserDetails, AuthenticationError};

/// Called by classes which make use of a UserDetailsService to check the status of the
/// loaded UserDetails object. Typically this will involve examining the various flags associated with the account and
/// raising an exception if the information cannot be used (for example if the user account is locked or disabled),
/// but a custom implementation could perform any checks it wished.
///
/// The intention is that this interface should only be used for checks on the persistent data associated with the user.
/// It should not involved in making any authentication decisions based on a submitted authentication request.
pub trait UserDetailsChecker
where
    Self: Send + Sync,
{
    /// Examines the User
    fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError>;
}
