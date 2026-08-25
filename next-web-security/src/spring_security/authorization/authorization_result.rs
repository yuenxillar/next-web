use std::any::Any;

use next_web_core::{clone_trait_object, DynClone};

pub trait AuthorizationResult
where
    Self: Send + Sync,
    Self: DynClone,
    Self: Any,
{
    fn is_granted(&self) -> bool;
}

clone_trait_object!(AuthorizationResult where Self: Send + Sync + Any);
