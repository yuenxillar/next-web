use std::any::Any;

/// Represents an authorization result
pub trait AuthorizationResult
where
    Self: Send + Sync,
    Self: Any,
{
    /// Returns:
    /// whether the access has been granted
    fn is_granted(&self) -> bool;
}
