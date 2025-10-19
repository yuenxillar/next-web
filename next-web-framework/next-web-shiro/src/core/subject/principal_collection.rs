use std::collections::HashSet;

use crate::core::object::Object;


pub trait PrincipalCollection
where 
Self: Iterator<Item = Object>,
Self: Send + Sync
{

    fn get_primary_principal<'a>(&self)-> Option<&'a Object>;

    // fn one_by_type<T>(&self) -> Option<& T>;

    // fn by_type<T>(&self) -> Vec<&T>;

    fn get_realm_names(&self) -> Option<HashSet<&str>>;

    fn is_empty(&self) -> bool;
}