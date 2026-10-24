use std::fmt::Debug;

use crate::core::poi::ss::{
    usermodel::conditional_formatting_rule::ConditionalFormattingRule,
    util::cell_range_address::CellRangeAddress,
};

/// The ConditionalFormatting class encapsulates all settings of Conditional Formatting.
///
/// The class can be used to make a copy ConditionalFormatting settings.
pub trait ConditionalFormatting: Debug {
    /// Get the array of `CellRangeAddress`es.
    ///
    /// # Returns
    /// Array of cell range addresses. Never empty.
    fn get_formatting_ranges(&self) -> &[CellRangeAddress];

    /// Sets the cell ranges the rule conditional formatting must be applied to.
    ///
    /// # Arguments
    /// * `ranges` - non-empty array of `CellRangeAddress`es
    fn set_formatting_ranges(&mut self, ranges: Vec<CellRangeAddress>);

    /// Replaces an existing Conditional Formatting rule at position idx.
    ///
    /// Excel pre-2007 allows to create up to 3 Conditional Formatting rules,
    /// 2007 and later allow unlimited numbers.
    /// This method can be useful to modify existing Conditional Formatting rules.
    ///
    /// # Arguments
    /// * `idx` - position of the rule. Should be between 0 and 2 for Excel before 2007, otherwise 0+.
    /// * `cf_rule` - Conditional Formatting rule
    ///
    /// # Panics
    /// Panics if idx is out of bounds.
    fn set_rule(&mut self, idx: usize, cf_rule: Box<dyn ConditionalFormattingRule>);

    /// Add a Conditional Formatting rule.
    ///
    /// Excel pre-2007 allows to create up to 3 Conditional Formatting rules.
    ///
    /// # Arguments
    /// * `cf_rule` - Conditional Formatting rule
    fn add_rule(&mut self, cf_rule: Box<dyn ConditionalFormattingRule>);

    /// Get the Conditional Formatting rule at position idx.
    ///
    /// # Arguments
    /// * `idx` - position of the rule
    ///
    /// # Returns
    /// The Conditional Formatting rule at position idx, or `None` if idx is out of bounds.
    fn get_rule(&self, idx: usize) -> Option<&dyn ConditionalFormattingRule>;

    /// Get the number of Conditional Formatting rules.
    ///
    /// # Returns
    /// Number of Conditional Formatting rules.
    fn get_number_of_rules(&self) -> usize;
}
