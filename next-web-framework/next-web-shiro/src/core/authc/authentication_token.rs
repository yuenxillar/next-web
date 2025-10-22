use std::{any::Any, fmt::Display};

use crate::core::object::Object;

pub trait AuthenticationToken
where 
Self: Send + Sync,
Self: Any + Display
{
    fn get_principal(&self) -> & Object;

    fn get_credentials(&self)-> Option<& Object>;
}