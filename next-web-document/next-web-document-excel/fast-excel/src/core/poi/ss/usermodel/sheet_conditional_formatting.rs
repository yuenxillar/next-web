use crate::core::poi::ss::{
    usermodel::{
        conditional_formatting::ConditionalFormatting,
        conditional_formatting_rule::ConditionalFormattingRule, extended_color::ExtendedColor,
        icon_set::IconSet,
    },
    util::cell_range_address::CellRangeAddress,
};

/// The 'Conditional Formatting' facet of `Sheet`.
pub trait SheetConditionalFormatting {
    /// Add a new Conditional Formatting to the sheet.
    ///
    /// # Arguments
    /// * `regions` - list of rectangular regions to apply conditional formatting rules
    /// * `rule` - the rule to apply
    ///
    /// # Returns
    /// Index of the newly created Conditional Formatting object
    fn add_conditional_formatting(
        &mut self,
        regions: &[CellRangeAddress],
        rule: &dyn ConditionalFormattingRule,
    ) -> usize;

    /// Add a new Conditional Formatting consisting of two rules.
    ///
    /// # Arguments
    /// * `regions` - list of rectangular regions to apply conditional formatting rules
    /// * `rule1` - the first rule
    /// * `rule2` - the second rule
    ///
    /// # Returns
    /// Index of the newly created Conditional Formatting object
    fn add_conditional_formatting_two_rules(
        &mut self,
        regions: &[CellRangeAddress],
        rule1: &dyn ConditionalFormattingRule,
        rule2: &dyn ConditionalFormattingRule,
    ) -> usize;

    /// Add a new Conditional Formatting set to the sheet.
    ///
    /// # Arguments
    /// * `regions` - list of rectangular regions to apply conditional formatting rules
    /// * `cf_rules` - set of up to conditional formatting rules (max 3 for Excel pre-2007)
    ///
    /// # Returns
    /// Index of the newly created Conditional Formatting object
    fn add_conditional_formatting_multiple_rules(
        &mut self,
        regions: &[CellRangeAddress],
        cf_rules: &[Box<dyn ConditionalFormattingRule>],
    ) -> usize;

    /// Adds a copy of a ConditionalFormatting object to the sheet.
    ///
    /// This method could be used to copy ConditionalFormatting object
    /// from one sheet to another.
    ///
    /// # Arguments
    /// * `cf` - the Conditional Formatting to clone
    ///
    /// # Returns
    /// Index of the new Conditional Formatting object
    fn add_conditional_formatting_object(&mut self, cf: &dyn ConditionalFormatting) -> usize;

    /// A factory method allowing to create a conditional formatting rule
    /// with a cell comparison operator.
    ///
    /// The created conditional formatting rule compares a cell value
    /// to a formula calculated result, using the specified operator.
    /// The type of the created condition is `ConditionType::CELL_VALUE_IS`.
    ///
    /// # Arguments
    /// * `comparison_operation` - MUST be a constant value from `ComparisonOperator`:
    ///   - BETWEEN
    ///   - NOT_BETWEEN
    ///   - EQUAL
    ///   - NOT_EQUAL
    ///   - GT
    ///   - LT
    ///   - GE
    ///   - LE
    /// * `formula1` - formula for the value, compared with the cell
    /// * `formula2` - second formula (only used with `BETWEEN` and `NOT_BETWEEN` operations)
    fn create_conditional_formatting_rule(
        &self,
        comparison_operation: u8,
        formula1: &str,
        formula2: Option<&str>,
    ) -> Box<dyn ConditionalFormattingRule>;

    /// Create a conditional formatting rule that compares a cell value
    /// to a formula calculated result, using an operator.
    ///
    /// The type of the created condition is `ConditionType::CELL_VALUE_IS`.
    ///
    /// # Arguments
    /// * `comparison_operation` - MUST be a constant value from `ComparisonOperator`
    ///   except BETWEEN and NOT_BETWEEN
    /// * `formula` - the formula to determine if the conditional formatting is applied
    fn create_conditional_formatting_rule_single_formula(
        &self,
        comparison_operation: u8,
        formula: &str,
    ) -> Box<dyn ConditionalFormattingRule>;

    /// Create a conditional formatting rule based on a Boolean formula.
    /// When the formula result is true, the cell is highlighted.
    ///
    /// The type of the created format condition is `ConditionType::FORMULA`.
    ///
    /// # Arguments
    /// * `formula` - the formula to evaluate. MUST be a Boolean function.
    fn create_conditional_formatting_rule_formula(
        &self,
        formula: &str,
    ) -> Box<dyn ConditionalFormattingRule>;

    /// Create a Databar conditional formatting rule.
    ///
    /// The thresholds and colour for it will be created, but will be
    /// empty and require configuring with
    /// `ConditionalFormattingRule::get_data_bar_formatting()`
    /// then
    /// `DataBarFormatting::get_min_threshold()`
    /// and
    /// `DataBarFormatting::get_max_threshold()`
    fn create_conditional_formatting_rule_data_bar(
        &self,
        color: &dyn ExtendedColor,
    ) -> Box<dyn ConditionalFormattingRule>;

    /// Create an Icon Set / Multi-State conditional formatting rule.
    ///
    /// The thresholds for it will be created, but will be empty
    /// and require configuring with
    /// `ConditionalFormattingRule::get_multi_state_formatting()`
    /// then
    /// `IconMultiStateFormatting::get_thresholds()`
    fn create_conditional_formatting_rule_icon_set(
        &self,
        icon_set: IconSet,
    ) -> Box<dyn ConditionalFormattingRule>;

    /// Create a Color Scale / Color Gradient conditional formatting rule.
    ///
    /// The thresholds and colours for it will be created, but will be
    /// empty and require configuring with
    /// `ConditionalFormattingRule::get_color_scale_formatting()`
    /// then
    /// `ColorScaleFormatting::get_thresholds()`
    /// and
    /// `ColorScaleFormatting::get_colors()`
    fn create_conditional_formatting_color_scale_rule(&self) -> Box<dyn ConditionalFormattingRule>;

    /// Gets Conditional Formatting object at a particular index.
    ///
    /// # Arguments
    /// * `index` - 0-based index of the Conditional Formatting object to fetch
    ///
    /// # Returns
    /// Conditional Formatting object or `None` if not found
    ///
    /// # Panics
    /// Panics if the index is outside of the allowable range (0 ... number_of_formats - 1)
    fn get_conditional_formatting_at(&self, index: usize) -> Option<&dyn ConditionalFormatting>;

    /// Gets the number of conditional formats in this sheet.
    ///
    /// # Returns
    /// The number of conditional formats in this sheet
    fn get_num_conditional_formattings(&self) -> usize;

    /// Removes a Conditional Formatting object by index.
    ///
    /// # Arguments
    /// * `index` - 0-based index of the Conditional Formatting object to remove
    ///
    /// # Panics
    /// Panics if the index is outside of the allowable range (0 ... number_of_formats - 1)
    fn remove_conditional_formatting(&mut self, index: usize);
}
