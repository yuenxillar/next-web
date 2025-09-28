use dyn_clone::DynClone;
use axum::Router;
use crate::{traits::group::Group, ApplicationContext};

pub trait UseRouter
where
    Self: Send + Sync,
    Self: Group + DynClone,
{
    fn use_router(&self, router: axum::Router, ctx: &mut ApplicationContext) -> Router;
}

dyn_clone::clone_trait_object!(UseRouter);