use next_web_core::error::BoxError;

use crate::core::{
    context::analysis_context::AnalysisContext, read::listener::read_listener::ReadListener,
};

#[derive(Clone)]
pub struct ReadListenerWrapper<T>(T);

impl<T> ReadListenerWrapper<T> {
    pub fn new(listener: T) -> Self {
        ReadListenerWrapper(listener)
    }
}

impl<T> ReadListener<T> for ReadListenerWrapper<T>
where
    Self: ReadListener<T>,
{
    fn on_error(
        &self,
        error: BoxError,
        context: &mut dyn AnalysisContext<T>,
    ) -> Result<(), BoxError> {
        self.0.on_error(error, context)
    }
}
