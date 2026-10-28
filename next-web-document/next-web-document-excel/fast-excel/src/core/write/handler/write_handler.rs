use next_web_core::{DynClone, clone_trait_object};

pub trait WriteHandler
where
    Self: 'static,
    Self: DynClone,
{
}

clone_trait_object!(WriteHandler);
