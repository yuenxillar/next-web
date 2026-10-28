use crate::core::poi::ss::usermodel::{
    border_formatting::BorderFormatting, excel_number_format::ExcelNumberFormat,
    font_formatting::FontFormatting, pattern_formatting::PatternFormatting,
};

/// Interface for classes providing differential style definitions, such as conditional format rules
/// and table/pivot table styles.
pub trait DifferentialStyleProvider: Send + Sync {
    /// Get border formatting object if defined
    ///
    /// # Returns
    /// * `Some` border formatting object if defined, `None` otherwise
    fn get_border_formatting(&self) -> Option<Box<dyn BorderFormatting>>;

    /// Get font formatting object if defined
    ///
    /// # Returns
    /// * `Some` font formatting object if defined, `None` otherwise
    fn get_font_formatting(&self) -> Option<Box<dyn FontFormatting>>;

    /// Get number format defined for this rule
    ///
    /// # Returns
    /// * `Some` number format if defined, `None` if the cell default should be used
    fn get_number_format(&self) -> Option<ExcelNumberFormat>;

    /// Get pattern formatting object if defined
    ///
    /// # Returns
    /// * `Some` pattern formatting object if defined, `None` otherwise
    fn get_pattern_formatting(&self) -> Option<Box<dyn PatternFormatting>>;

    /// Get the number of rows or columns in a band or stripe.
    /// For styles that represent stripes, it must be > 1, for all others it is 0.
    /// Not the greatest overloading by the OOXML spec.
    ///
    /// # Returns
    /// * Number of rows/columns in a stripe for stripe styles, 0 for all others
    fn get_stripe_size(&self) -> u32;
}
