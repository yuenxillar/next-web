use dyn_clone::DynClone;

use crate::ApplicationContext;

pub trait ApplyLayer: DynClone + Send + Sync {
    fn order(&self) -> u32 {
        100
    }

    fn apply_layer(&self, router: &mut axum::Router, ctx: &mut ApplicationContext);
}

dyn_clone::clone_trait_object!(ApplyLayer);