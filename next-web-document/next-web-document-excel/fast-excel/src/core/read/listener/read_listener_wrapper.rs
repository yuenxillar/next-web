use next_web_core::error::BoxError;

use crate::core::{
    context::analysis_context::AnalysisContext, event::listener::Listener,
    read::listener::read_listener::ReadListener,
};

#[derive(Clone)]
pub struct ReadListenerWrapper<T>(T);

impl<T> ReadListenerWrapper<T> {
    pub fn new(listener: T) -> Self {
        ReadListenerWrapper(listener)
    }
}

impl<T, L> ReadListener<L> for ReadListenerWrapper<T>
where
    T: ReadListener<L>,
    T: Clone,
{
    fn on_error(
        &self,
        error: BoxError,
        context: &mut dyn AnalysisContext<L>,
    ) -> Result<(), BoxError> {
        self.0.on_error(error, context)
    }
}

impl<T> Listener for ReadListenerWrapper<T> where T: Listener {}
