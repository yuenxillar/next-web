use std::any::Any;

use dyn_clone::{DynClone, clone_trait_object};

pub trait ApplicationEvent
where
    Self: Send + Sync,
    Self: Any,
    Self: DynClone,
{
    fn timestamp(&self) -> u64 {
        0
    }

    fn source(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

clone_trait_object!(ApplicationEvent);
