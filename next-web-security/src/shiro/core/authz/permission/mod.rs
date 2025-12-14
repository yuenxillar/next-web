use std::fmt::Display;
use std::hash::Hasher;
use std::hash::Hash;


pub mod wildcard_permission_resolver;
pub mod role_permission_resolver_aware;
pub mod permission_resolver_aware;
pub mod role_permission_resolver;
pub mod permission_resolver;

pub trait Permission
where 
Self: Send + Sync,
Self: Display,
{
    fn implies(&self, p: &dyn Permission) -> bool;

    fn identifier(&self) -> &str;
}

struct PermissionWrapper(Box<dyn Permission>);


impl PartialEq for PermissionWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.identifier() == other.0.identifier()
    }
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