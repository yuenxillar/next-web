use next_web_core::{DynClone, clone_trait_object, error::BoxError};

use crate::core::{context::analysis_context::AnalysisContext, event::listener::Listener};

pub trait ReadListener<T>
where
    Self: Listener,
    Self: DynClone,
{
    /// All listeners receive this method when any one Listener does an error report. If an exception is thrown here, the
    /// entire read will terminate.
    fn on_error(
        &self,
        error: BoxError,
        _context: &mut dyn AnalysisContext<T>,
    ) -> Result<(), BoxError> {
        Err(error)
    }

    fn invoke(&mut self, data: &T, context: &mut dyn AnalysisContext<T>) -> Result<(), BoxError>;

    fn do_after_all_analysed(&mut self, context: &mut dyn AnalysisContext<T>);
}

clone_trait_object!(<T> ReadListener<T> where Self: Listener, Self: DynClone);
