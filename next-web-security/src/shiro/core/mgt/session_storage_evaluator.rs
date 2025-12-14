use std::any::Any;

use next_web_core::{async_trait, clone_trait_object, DynClone};

use crate::core::subject::Subject;

#[async_trait]
pub trait SessionStorageEvaluator
where
    Self: Send + Sync,
    Self: Any + DynClone,
{
    async fn is_session_storage_enabled(&self, subject: &dyn Subject) -> bool;
}

clone_trait_object!(SessionStorageEvaluator);
