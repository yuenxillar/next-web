use crate::core::poi::ss::formula::evaluation_cell::EvaluationCell;

/// Evaluation sheet interface
pub trait EvaluationSheet {
    /// Returns `None` if there is no cell at the specified coordinates
    fn get_cell(&self, row_index: i32, column_index: i32) -> Option<&dyn EvaluationCell>;

    /// Propagated from `EvaluationWorkbook::clear_all_cached_result_values()` to clear locally cached data.
    ///
    /// See also:
    /// - `WorkbookEvaluator::clear_all_cached_result_values()`
    /// - `EvaluationWorkbook::clear_all_cached_result_values()`
    fn clear_all_cached_result_values(&mut self);

    /// Returns last row index referenced on this sheet, for evaluation optimization
    fn get_last_row_num(&self) -> i32;

    /// Used by SUBTOTAL and similar functions that have options to ignore hidden rows
    /// Returns `true` if the row is hidden, `false` if not
    fn is_row_hidden(&self, row_index: i32) -> bool;
}
