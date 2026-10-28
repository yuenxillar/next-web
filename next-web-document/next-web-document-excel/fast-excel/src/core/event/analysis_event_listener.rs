use std::collections::HashMap;

use crate::core::{
    context::analysis_context::AnalysisContext, read::listener::read_listener::ReadListener,
};

/// Receives the return of each piece of data parsed
pub trait AnalysisEventListener<T>
where
    Self: ReadListener<T>,
{
    /// Returns the header as a map.Override the current method to receive header data.
    #[allow(unused_variables)]
    fn invoke_head_map(&self, head_map: HashMap<u32, String>, context: &dyn AnalysisContext<T>) {}
}
