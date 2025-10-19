use std::any::Any;

use crate::core::object::Object;

pub trait AuthenticationToken
where 
Self: Send,
Self: Any
{
    fn get_principal(&self) -> & Object;

    fn get_credentials(&self)-> Option<& Object>;
}