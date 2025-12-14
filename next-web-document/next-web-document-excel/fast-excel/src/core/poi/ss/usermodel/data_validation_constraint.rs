/// Represents a data validation constraint for cells.
pub trait DataValidationConstraint {
    /// Gets the data validation type of this constraint.
    ///
    /// # Returns
    /// Data validation type.
    fn get_validation_type(&self) -> i32;

    /// Gets the operator used for this constraint.
    ///
    /// # Returns
    /// The operator.
    fn get_operator(&self) -> i32;

    /// Sets the comparison operator for this constraint.
    ///
    /// # Arguments
    /// * `operator` - The operator to set.
    fn set_operator(&mut self, operator: i32);

    /// If validation type is `validation_type::LIST` and `formula1` was
    /// comma-separated literal values rather than a range or named range,
    /// returns list of literal values.
    /// Otherwise returns `None`.
    ///
    /// # Returns
    /// Array of explicit list values or `None`.
    fn get_explicit_list_values(&self) -> Option<Vec<String>>;

    /// Sets explicit list values.
    ///
    /// # Arguments
    /// * `explicit_list_values` - Array of explicit list values.
    fn set_explicit_list_values(&mut self, explicit_list_values: Vec<String>);

    /// Gets the formula for expression 1.
    ///
    /// # Returns
    /// The formula for expression 1, or `None`.
    fn get_formula1(&self) -> Option<&str>;

    /// Sets a formula for expression 1.
    ///
    /// # Arguments
    /// * `formula1` - The formula for expression 1.
    fn set_formula1(&mut self, formula1: String);

    /// Gets the formula for expression 2.
    ///
    /// # Returns
    /// The formula for expression 2, or `None`.
    fn get_formula2(&self) -> Option<&str>;

    /// Sets a formula for expression 2.
    ///
    /// # Arguments
    /// * `formula2` - The formula for expression 2.
    fn set_formula2(&mut self, formula2: String);
}

/// Condition operator enum
pub mod operator_type {
    pub const BETWEEN: i32 = 0x00;
    pub const NOT_BETWEEN: i32 = 0x01;
    pub const EQUAL: i32 = 0x02;
    pub const NOT_EQUAL: i32 = 0x03;
    pub const GREATER_THAN: i32 = 0x04;
    pub const LESS_THAN: i32 = 0x05;
    pub const GREATER_OR_EQUAL: i32 = 0x06;
    pub const LESS_OR_EQUAL: i32 = 0x07;
    /// default value to supply when the operator type is not used
    pub const IGNORED: i32 = BETWEEN;

    /// Validates that a second argument is provided when needed.
    ///
    /// # Arguments
    /// * `comparison_operator` - The comparison operator.
    /// * `param_value` - The second parameter value.
    ///
    /// # Panics
    /// Panics if `param_value` is `None` for operators that require it.
    pub fn validate_second_arg(comparison_operator: i32, param_value: Option<&String>) {
        match comparison_operator {
            BETWEEN | NOT_BETWEEN => {
                if param_value.is_none() {
                    panic!("expr2 must be supplied for 'between' comparisons");
                }
            }
            _ => {
                // all other operators don't need second arg
            }
        }
    }
}

/// ValidationType enum
pub mod validation_type {
    /// 'Any value' type - value not restricted
    pub const ANY: i32 = 0x00;
    /// Integer ('Whole number') type
    pub const INTEGER: i32 = 0x01;
    /// Decimal type
    pub const DECIMAL: i32 = 0x02;
    /// List type (combo box type)
    pub const LIST: i32 = 0x03;
    /// Date type
    pub const DATE: i32 = 0x04;
    /// Time type
    pub const TIME: i32 = 0x05;
    /// String length type
    pub const TEXT_LENGTH: i32 = 0x06;
    /// Formula ('Custom') type
    pub const FORMULA: i32 = 0x07;
}
