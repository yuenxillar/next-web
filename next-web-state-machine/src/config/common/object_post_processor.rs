use next_web_core::{DynClone, anys::any_value::AnyValue, clone_trait_object};

/// A trait for post-processing objects
pub trait ObjectPostProcessor<T = AnyValue>
where
    Self: Send + Sync,
    Self: DynClone,
    Self: 'static,
{
    /// Post-process an object
    fn post_process(&self, object: &mut T);
}

clone_trait_object!(<T> ObjectPostProcessor<T> where T: Clone + Send + Sync + 'static);

/// A quiescent (no-op) object post processor
#[derive(Clone)]
pub struct QuiescentPostProcessor;

impl ObjectPostProcessor for QuiescentPostProcessor {
    #[allow(unused_variables)]
    fn post_process(&self, object: &mut AnyValue) {}
}
