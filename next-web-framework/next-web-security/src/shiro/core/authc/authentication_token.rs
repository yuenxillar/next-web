use std::{any::Any, fmt::Display};

use next_web_core::DynClone;

use crate::core::util::object::Object;

pub trait AuthenticationToken
where 
Self: Send + Sync,
Self: Any + Display + DynClone
{
    fn get_principal(&self) -> Object;

    fn get_credentials(&self)-> Option<Object>;
}

next_web_core::clone_trait_object!(AuthenticationToken);