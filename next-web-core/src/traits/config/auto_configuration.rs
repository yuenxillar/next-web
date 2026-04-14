use std::error::Error;

use async_trait::async_trait;
use dyn_clone::{DynClone, clone_trait_object};

use crate::ApplicationContext;

#[async_trait]
pub trait AutoConfiguration
where
    Self: Send + Sync,
    Self: 'static,
    Self: DynClone,
{
    fn order(&self) -> i32 {
        100
    }

    async fn configuration(&mut self, ctx: &mut ApplicationContext) -> Result<(), Box<dyn Error>>;
}

clone_trait_object!(AutoConfiguration where Self: Send + Sync);
