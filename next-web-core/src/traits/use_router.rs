use crate::{ApplicationContext, traits::group::Group};
use axum::Router;
use dyn_clone::DynClone;

pub trait UseRouter
where
    Self: Send + Sync,
    Self: Group + DynClone,
{
    fn use_router(&self, router: axum::Router, ctx: &mut ApplicationContext) -> Router;
}

dyn_clone::clone_trait_object!(UseRouter);
