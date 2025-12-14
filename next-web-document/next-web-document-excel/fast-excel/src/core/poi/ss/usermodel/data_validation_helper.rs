use crate::core::poi::ss::{
    usermodel::{
        data_validation::DataValidation, data_validation_constraint::DataValidationConstraint,
    },
    util::cell_range_address_list::CellRangeAddressList,
};

/// Trait for creating data validation constraints and validations
pub trait DataValidationHelper {
    /// Create a formula list constraint
    ///
    /// # Arguments
    /// * `list_formula` - The formula that defines the list values
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_formula_list_constraint(
        &self,
        list_formula: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create an explicit list constraint
    ///
    /// # Arguments
    /// * `list_of_values` - Array of string values for the list
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_explicit_list_constraint(
        &self,
        list_of_values: &[String],
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a numeric constraint
    ///
    /// # Arguments
    /// * `validation_type` - Type of validation
    /// * `operator_type` - Operator type (e.g., between, greater than, etc.)
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_numeric_constraint(
        &self,
        validation_type: i32,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a text length constraint
    ///
    /// # Arguments
    /// * `operator_type` - Operator type
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_text_length_constraint(
        &self,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a decimal constraint
    ///
    /// # Arguments
    /// * `operator_type` - Operator type
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_decimal_constraint(
        &self,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create an integer constraint
    ///
    /// # Arguments
    /// * `operator_type` - Operator type
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_integer_constraint(
        &self,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a date constraint
    ///
    /// # Arguments
    /// * `operator_type` - Operator type
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    /// * `date_format` - Date format string
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_date_constraint(
        &self,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
        date_format: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a time constraint
    ///
    /// # Arguments
    /// * `operator_type` - Operator type
    /// * `formula1` - First formula/value
    /// * `formula2` - Second formula/value (for between operators)
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_time_constraint(
        &self,
        operator_type: i32,
        formula1: &str,
        formula2: &str,
    ) -> Box<dyn DataValidationConstraint>;

    /// Create a custom constraint
    ///
    /// # Arguments
    /// * `formula` - Custom formula for validation
    ///
    /// # Returns
    /// * Data validation constraint
    fn create_custom_constraint(&self, formula: &str) -> Box<dyn DataValidationConstraint>;

    /// Create a data validation
    ///
    /// # Arguments
    /// * `constraint` - Data validation constraint
    /// * `cell_range_address_list` - Range of cells to apply the validation to
    ///
    /// # Returns
    /// * Data validation object
    fn create_validation(
        &self,
        constraint: Box<dyn DataValidationConstraint>,
        cell_range_address_list: CellRangeAddressList,
    ) -> Box<dyn DataValidation>;
}
