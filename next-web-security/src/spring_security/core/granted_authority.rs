use std::{
    any::Any,
    hash::{Hash, Hasher},
};

/// Represents an authority granted to an Authentication object.
/// A GrantedAuthority must either represent itself as a String or be specifically supported by an
/// AuthorizationManager.
pub trait GrantedAuthority
where
    Self: Send + Sync,
    Self: Any,
{
    /// Returns the string representation of this authority.
    ///
    /// If the `GrantedAuthority` can be represented as a `String` and that `String`
    /// is sufficient in precision to be relied upon for an access control decision
    /// by an `AuthorizationManager` (or delegate), this method should return such
    /// a `String`.
    ///
    /// If the `GrantedAuthority` cannot be expressed with sufficient precision as a
    /// `String`, `None` should be returned. Returning `None` will require an
    /// `AccessDecisionManager` (or delegate) to specifically support the
    /// `GrantedAuthority` implementation, so returning `None` should be avoided
    /// unless actually required.
    ///
    /// # Returns
    /// A representation of the granted authority (or `None` if the granted authority
    /// cannot be expressed as a `String` with sufficient precision).
    fn authority(&self) -> Option<&str>;
}

impl Hash for dyn GrantedAuthority {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.authority().hash(state);
    }
}

impl PartialEq for dyn GrantedAuthority {
    fn eq(&self, other: &Self) -> bool {
        self.authority() == other.authority()
    }
}

impl Eq for dyn GrantedAuthority {}
