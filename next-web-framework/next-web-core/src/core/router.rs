use axum::Router;
use dyn_clone::DynClone;

use crate::ApplicationContext;



pub trait ApplyRouter: DynClone + Send + Sync{
    
    fn router(&self, ctx: &mut ApplicationContext) -> Router;
}

dyn_clone::clone_trait_object!(ApplyRouter);