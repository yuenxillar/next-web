use dyn_clone::DynClone;

use crate::ApplicationContext;

pub trait UseRouter
where
    Self: DynClone + Send + Sync,
{
    fn use_router(&self, router: axum::Router, ctx: &mut ApplicationContext) -> axum::Router;

    fn group(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

dyn_clone::clone_trait_object!(UseRouter);
