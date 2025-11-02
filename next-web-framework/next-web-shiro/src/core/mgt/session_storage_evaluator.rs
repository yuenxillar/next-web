use std::any::Any;

use next_web_core::{clone_trait_object, DynClone};

use crate::core::subject::Subject;


pub trait SessionStorageEvaluator
where 
Self: Send + Sync,
Self: Any + DynClone
{
    fn is_session_storage_enabled(&self, subject: &dyn Subject) -> bool;
}


clone_trait_object!(SessionStorageEvaluator);