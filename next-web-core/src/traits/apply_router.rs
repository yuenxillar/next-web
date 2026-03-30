use dyn_clone::DynClone;

use crate::ApplicationContext;

pub trait ApplyRouter
where
    Self: DynClone,
    Self: Send + Sync,
{
    fn order(&self) -> i32 {
        i32::MAX
    }

    fn apply(&mut self, ctx: &mut ApplicationContext) -> axum::Router;
}

dyn_clone::clone_trait_object!(ApplyRouter);
