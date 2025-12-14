use dyn_clone::DynClone;

use crate::ApplicationContext;

pub trait ApplyRouter: DynClone + Send + Sync {

    fn order(&self) -> i32 {
        i32::MAX
    }

    fn router(&self, ctx: &mut ApplicationContext) -> axum::Router;
}

dyn_clone::clone_trait_object!(ApplyRouter);
