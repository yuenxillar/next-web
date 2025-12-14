use std::fmt::Debug;

use next_web_core::{DynClone, clone_trait_object};

pub trait Converter
where
    Self: 'static + DynClone,
    Self: Debug,
{
}

clone_trait_object!(Converter);
