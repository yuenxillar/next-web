use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::{ApplicationContext, error::BoxError};

#[async_trait]
pub trait AutoConfiguration
where
    Self: Send + Sync,
    Self: 'static,
    Self: DynClone,
{
    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), BoxError>;
}

clone_trait_object!(AutoConfiguration where Self: Send + Sync);
