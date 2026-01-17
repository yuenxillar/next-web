use next_web_core::error::BoxError;

use crate::core::{
    context::analysis_context::AnalysisContext,
    event::{analysis_event_listener::AnalysisEventListener, listener::Listener},
    read::listener::read_listener::ReadListener,
};

#[derive(Clone)]
pub struct SyncReadListener<T> {
    list: Vec<T>,
}

impl<T> SyncReadListener<T>
where
    T: Clone,
{
    pub fn get_list(&self) -> Vec<&T> {
        self.list.iter().collect()
    }

    pub fn take_all(&mut self) -> Vec<T> {
        std::mem::take(&mut self.list)
    }

    pub fn set_list(&mut self, list: Vec<T>) {
        self.list = list;
    }
}

impl<T> AnalysisEventListener<T> for SyncReadListener<T> where T: Clone {}

impl<T> ReadListener<T> for SyncReadListener<T>
where
    T: Clone,
{
    fn invoke(&mut self, data: &T, _context: &mut dyn AnalysisContext<T>) -> Result<(), BoxError> {
        self.list.push(data.clone());
        Ok(())
    }

    fn do_after_all_analysed(&mut self, _context: &mut dyn AnalysisContext<T>) {}
}

impl<T> Listener for SyncReadListener<T> {}

impl<T> Default for SyncReadListener<T> {
    fn default() -> Self {
        SyncReadListener { list: Vec::new() }
    }
}
