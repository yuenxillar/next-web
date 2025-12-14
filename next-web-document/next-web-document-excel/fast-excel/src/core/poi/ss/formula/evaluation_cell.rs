use next_web_core::anys::any_value::AnyValue;

use crate::core::{
    enums::cell_data::CellType,
    poi::ss::{
        formula::evaluation_sheet::EvaluationSheet, util::cell_range_address::CellRangeAddress,
    },
};

/// Abstracts a cell for the purpose of formula evaluation
pub trait EvaluationCell {
    /// Returns an object that identifies the underlying cell,
    /// suitable for use as a key in a HashMap
    fn get_identity_key(&self) -> Option<&AnyValue>;

    fn get_sheet(&self) -> Option<&dyn EvaluationSheet>;
    fn get_row_index(&self) -> i32;
    fn get_column_index(&self) -> i32;
    fn get_cell_type(&self) -> CellType;

    fn get_numeric_cell_value(&self) -> f64;
    fn get_string_cell_value(&self) -> String;
    fn get_boolean_cell_value(&self) -> bool;
    fn get_error_cell_value(&self) -> i32;

    /// Returns the range address if this cell is part of an array formula
    fn get_array_formula_range(&self) -> Option<&CellRangeAddress>;

    /// Returns whether this cell is part of an array formula group
    fn is_part_of_array_formula_group(&self) -> bool;

    /// Returns cell type of cached formula result
    fn get_cached_formula_result_type(&self) -> CellType;
}
