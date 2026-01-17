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

impl<T, T1> ReadListener<T1> for ReadListenerWrapper<T>
where
    T: ReadListener<T1>,
    T: Clone,
{
    fn on_error(
        &self,
        error: BoxError,
        context: &mut dyn AnalysisContext<T1>,
    ) -> Result<(), BoxError> {
        self.0.on_error(error, context)
    }

    fn invoke(&mut self, data: &T1, context: &mut dyn AnalysisContext<T1>) -> Result<(), BoxError> {
        self.0.invoke(data, context)
    }

    fn do_after_all_analysed(&mut self, context: &mut dyn AnalysisContext<T1>) {
        self.0.do_after_all_analysed(context)
    }
}

impl<T> Listener for ReadListenerWrapper<T> where T: Listener {}
