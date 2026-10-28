use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;

pub mod all_permission;
pub mod permission_resolver;
pub mod permission_resolver_aware;
pub mod role_permission_resolver;
pub mod role_permission_resolver_aware;
pub mod wildcard_permission;
pub mod wildcard_permission_resolver;

pub use all_permission::AllPermission;
pub use wildcard_permission::WildcardPermission;
pub use wildcard_permission_resolver::WildcardPermissionResolver;

/// A right an application protects, and what it implies.
///
/// Shiro expresses a permission as a wildcard string and lets a permission
/// imply another one: `printer:*` implies `printer:print`, and `printer` implies
/// every permission of the printer domain.
pub trait Permission
where
    Self: Send + Sync,
    Self: Display,
{
    /// Returns whether this permission implies the given one.
    ///
    /// # Arguments
    ///
    /// * `p` - The permission a caller asks for.
    fn implies(&self, p: &dyn Permission) -> bool;

    /// Returns the string this permission was built from.
    fn identifier(&self) -> &str;
}


impl PartialEq for Box<dyn Permission> {
    fn eq(&self, other: &Self) -> bool {
        self.identifier() == other.identifier()
    }
}

impl Eq for Box<dyn Permission> {}

impl Hash for Box<dyn Permission> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.identifier().hash(state)
    }
}
