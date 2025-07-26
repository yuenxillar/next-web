use std::u32;

use dyn_clone::DynClone;

use crate::BoxFuture;

pub trait RequestMiddleware: DynClone + Send + Sync {
    fn order(&self) -> u32 {
        u32::MAX
    }

    // fn middleware(&self) -> BoxFuture;
}

dyn_clone::clone_trait_object!(RequestMiddleware);
