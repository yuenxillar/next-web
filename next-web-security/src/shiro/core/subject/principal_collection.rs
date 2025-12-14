use next_web_core::{traits::id::Id, DynClone};
use std::{collections::HashSet, fmt::Display};

use crate::core::util::object::Object;

pub trait PrincipalCollection
where
    Self: Iterator<Item = Object>,
    Self: Send + Sync,
    Self: Display,
    Self: DynClone + Id
{
    fn get_primary_principal<'a>(&self) -> Option<&'a Object>;

    // fn one_by_type<T>(&self) -> Option<& T>;

    // fn by_type<T>(&self) -> Vec<&T>;

    // fn from_realm(&self, realm_name: &str) -> Vec<>;

    fn get_realm_names(&self) -> Option<HashSet<&str>>;

    fn is_empty(&self) -> bool;
}

next_web_core::clone_trait_object!(PrincipalCollection);
