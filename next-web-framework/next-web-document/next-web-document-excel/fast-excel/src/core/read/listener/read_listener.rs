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
        context: &mut dyn AnalysisContext<T>,
    ) -> Result<(), BoxError>;
}

clone_trait_object!(<T> ReadListener<T> where Self: Listener, Self: DynClone);
