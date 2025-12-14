use crate::core::poi::ss::formula::workbook_evaluator::WorkbookEvaluator;

pub trait WorkbookEvaluatorProvider {
    /// Provide the underlying WorkbookEvaluator
    fn _get_workbook_evaluator(&self) -> &WorkbookEvaluator;
}
