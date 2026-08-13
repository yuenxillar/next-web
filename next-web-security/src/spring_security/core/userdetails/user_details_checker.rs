use crate::core::{userdetails::UserDetails, AuthenticationError};

pub trait UserDetailsChecker
where
    Self: Send + Sync,
{
    fn check(&self, to_check: &dyn UserDetails) -> Result<(), AuthenticationError>;
}
