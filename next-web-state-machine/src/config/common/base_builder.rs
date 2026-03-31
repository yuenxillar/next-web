use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use next_web_core::error::BoxError;
use next_web_core::traits::required::Required;

use crate::config::common::builder::Builder;

/// A base `AnnotationBuilder` that ensures the object being built is only built once
#[derive(Clone)]
pub struct BaseBuilder<O> {
    /// Flag tracking if the object has been built
    building: Arc<AtomicBool>,

    /// Built object is stored here (wrapped in Mutex for interior mutability)
    object: Option<O>,
}

impl<O> BaseBuilder<O>
where
    O: Clone,
{
    /// Creates a new `AbstractAnnotationBuilder`
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_object(&self) -> Option<O> {
        if !self.building.load(Ordering::Acquire) {
            return None;
        }

        self.object.clone()
    }
}

impl<O> Default for BaseBuilder<O> {
    fn default() -> Self {
        Self {
            building: Arc::new(Default::default()),
            object: None,
        }
    }
}

impl<T, O> Builder<O> for T
where
    T: Required<BaseBuilder<O>>,
    T: BaseBuilderExt<O>,
    O: Clone,
{
    fn build(&mut self) -> Result<O, BoxError> {
        // Try to set building flag from false to true
        if self
            .get_object()
            .building
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            // We successfully acquired the building lock
            let object = self.do_build()?;
            self.get_mut_object().object.replace(object.clone());

            Ok(object)
        } else {
            Err("This object has already been built".into())
        }
    }
}

pub trait BaseBuilderExt<O> {
    fn do_build(&mut self) -> Result<O, BoxError>;
}
