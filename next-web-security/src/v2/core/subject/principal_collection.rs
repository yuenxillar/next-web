use next_web_core::{DynClone, Id};
use std::{collections::HashSet, fmt::Display};

use crate::core::util::object::Object;

/// The principals of a subject, grouped by the realm that authenticated them.
///
/// A subject is identified by at least one principal — a user name, an account
/// identifier, a certificate subject — and a login can add the principals of
/// every realm that authenticated it.
pub trait PrincipalCollection
where
    Self: Send + Sync,
    Self: Display,
    Self: DynClone + Id,
{
    /// Returns the principal that identifies the subject.
    fn get_primary_principal(&self) -> Option<&Object>;

    /// Returns the names of the realms that contributed a principal.
    fn get_realm_names(&self) -> Option<HashSet<&str>>;

    /// Returns whether the collection holds no principal at all.
    fn is_empty(&self) -> bool;
}

next_web_core::clone_trait_object!(PrincipalCollection);
