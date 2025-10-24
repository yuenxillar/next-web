use std::{any::Any, fmt::Display};

use next_web_core::DynClone;

use crate::core::{object::Object, subject::principal_collection::PrincipalCollection};

pub trait AuthenticationInfo
where
    Self: Send + Sync,
    Self: Display + Any,
    Self: DynClone
{

    fn get_principals(&self) -> Option<&dyn PrincipalCollection>;


    fn get_credentials(&self) -> Option<& Object>;
}

next_web_core::clone_trait_object!(AuthenticationInfo);