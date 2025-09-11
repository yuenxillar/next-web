pub mod back_off_interrupted_error;
pub mod retry_error;

pub trait CloneableError: std::error::Error + Send + Sync + dyn_clone::DynClone {
    fn to_boxed(&self) -> Box<dyn CloneableError>;
}

impl<T> CloneableError for T
where
    T: std::error::Error + Clone + Send + Sync + 'static,
{
    fn to_boxed(&self) -> Box<dyn CloneableError> {
        Box::new(self.clone())
    }
}

dyn_clone::clone_trait_object!(CloneableError);
