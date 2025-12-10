use crate::core::{
    context::analysis_context::AnalysisContext,
    read::processor::analysis_event_processor::AnalysisEventProcessor,
};

pub struct DefaultAnalysisEventProcessor<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> DefaultAnalysisEventProcessor<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    fn deal_extra(&self, analysis_context: &mut dyn AnalysisContext<T>) {
        for read_listener in analysis_context.current_read_holder().read_listener_list() {
            // read_listener.ex
        }
        todo!()
    }
}

impl<T> AnalysisEventProcessor<T> for DefaultAnalysisEventProcessor<T> {
    fn extra(&self, analysis_context: &mut dyn AnalysisContext<T>) {
        self.deal_extra(analysis_context);
    }

    fn end_row(&self, analysis_context: &mut dyn AnalysisContext<T>) {
        todo!()
    }

    fn end_sheet(&self, analysis_context: &mut dyn AnalysisContext<T>) {
        todo!()
    }
}
