use std::collections::HashMap;

use crate::core::{
    enums::cell_data::CellType,
    poi::ss::usermodel::{cell::Cell, cell_value::CellValue},
};

/// Evaluates formula cells.
///
/// For performance reasons, this class keeps a cache of all previously calculated intermediate
/// cell values. Be sure to call `clear_all_cached_result_values()` if any workbook cells are changed between
/// calls to evaluate methods on this class.
pub trait FormulaEvaluator {
    /// Should be called whenever there are changes to input cells in the evaluated workbook.
    /// Failure to call this method after changing cell values will cause incorrect behaviour
    /// of the evaluate methods of this class
    fn clear_all_cached_result_values(&mut self);

    /// Should be called to tell the cell value cache that the specified (value or formula) cell
    /// has changed.
    /// Failure to call this method after changing cell values will cause incorrect behaviour
    /// of the evaluate methods of this class
    fn notify_set_formula(&mut self, cell: &dyn Cell);

    /// Should be called to tell the cell value cache that the specified cell has just become a
    /// formula cell, or the formula text has changed
    fn notify_delete_cell(&mut self, cell: &dyn Cell);

    /// Should be called to tell the cell value cache that the specified (value or formula) cell
    /// has changed.
    /// Failure to call this method after changing cell values will cause incorrect behaviour
    /// of the evaluate methods of this class
    fn notify_update_cell(&mut self, cell: &dyn Cell);

    /// Loops over all cells in all sheets of the associated workbook.
    /// For cells that contain formulas, their formulas are evaluated,
    /// and the results are saved. These cells remain as formula cells.
    /// For cells that do not contain formulas, no changes are made.
    /// This is a helpful wrapper around looping over all cells, and
    /// calling `evaluate_formula_cell` on each one.
    fn evaluate_all(&mut self);

    /// If cell contains a formula, the formula is evaluated and returned,
    /// else the CellValue simply copies the appropriate cell value from
    /// the cell and also its cell type. This method should be preferred over
    /// `evaluate_in_cell()` when the call should not modify the contents of the
    /// original cell.
    ///
    /// # Arguments
    /// * `cell` - The Cell to evaluate
    fn evaluate(&mut self, cell: &dyn Cell) -> CellValue;

    /// If cell contains formula, it evaluates the formula,
    /// and saves the result of the formula. The cell
    /// remains as a formula cell.
    /// Else if cell does not contain formula, this method leaves
    /// the cell unchanged.
    /// Note that the type of the formula result is returned,
    /// so you know what kind of value is also stored with
    /// the formula.
    ///
    /// ```rust
    /// let evaluated_cell_type = evaluator.evaluate_formula_cell(cell);
    /// ```
    ///
    /// Be aware that your cell will hold both the formula,
    /// and the result. If you want the cell replaced with
    /// the result of the formula, use `evaluate_in_cell()`
    ///
    /// # Arguments
    /// * `cell` - The cell to evaluate
    ///
    /// # Returns
    /// The type of the formula result, i.e. `None` if the cell is not a formula,
    /// or one of `CellType::Numeric`, `CellType::String`,
    /// `CellType::Boolean`, `CellType::Error`
    /// Note: the cell's type remains as `CellType::Formula` however.
    fn evaluate_formula_cell(&mut self, cell: &mut dyn Cell) -> Option<CellType>;

    /// If cell contains formula, it evaluates the formula, and
    /// puts the formula result back into the cell, in place
    /// of the old formula.
    /// Else if cell does not contain formula, this method leaves
    /// the cell unchanged.
    /// Note that the same instance of Cell is returned to
    /// allow chained calls like:
    ///
    /// ```rust
    /// let evaluated_cell_type = evaluator.evaluate_in_cell(&mut cell).cell_type();
    /// ```
    ///
    /// Be aware that your cell value will be changed to hold the
    /// result of the formula. If you simply want the formula
    /// value computed for you, use `evaluate_formula_cell()`
    ///
    /// # Arguments
    /// * `cell` - The Cell to evaluate and modify.
    fn evaluate_in_cell(&mut self, cell: &mut dyn Cell) -> &mut dyn Cell;

    /// Sets up the Formula Evaluator to be able to reference and resolve
    /// links to other workbooks, eg [Test.xls]Sheet1!A1.
    ///
    /// For a workbook referenced as [Test.xls]Sheet1!A1, you should
    /// supply a map containing the key "Test.xls" (no square brackets),
    /// and an open FormulaEvaluator onto that Workbook.
    ///
    /// # Arguments
    /// * `workbooks` - Map of workbook names (no square brackets) to an evaluator on that workbook
    fn setup_referenced_workbooks(&mut self, workbooks: HashMap<String, Box<dyn FormulaEvaluator>>);

    /// Whether to ignore missing references to external workbooks and
    /// use cached formula results in the main workbook instead.
    ///
    /// In some cases external workbooks referenced by formulas in the main workbook are not available.
    /// With this method you can control how POI handles such missing references:
    /// * by default ignore_missing_workbooks=false and POI throws an error
    ///   if an external reference cannot be resolved
    /// * if ignore_missing_workbooks=true then POI uses cached formula result
    ///   that already exists in the main workbook
    ///
    /// # Arguments
    /// * `ignore` - whether to ignore missing references to external workbooks
    fn set_ignore_missing_workbooks(&mut self, ignore: bool);

    /// Perform detailed output of formula evaluation for next evaluation only?
    /// Is for developer use only (also developers using POI for their XLS files).
    /// Log-Level WARN is for basic info, INFO for detailed information. These quite
    /// high levels are used because you have to explicitly enable this specific logging.
    ///
    /// # Arguments
    /// * `value` - whether to perform detailed output
    fn set_debug_evaluation_output_for_next_eval(&mut self, value: bool);
}
