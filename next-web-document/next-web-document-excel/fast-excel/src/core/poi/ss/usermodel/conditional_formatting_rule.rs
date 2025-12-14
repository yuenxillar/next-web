use std::fmt::Debug;

use crate::core::poi::ss::usermodel::{
    border_formatting::BorderFormatting, color_scale_formatting::ColorScaleFormatting,
    condition_filter_data::ConditionFilterData, condition_filter_type::ConditionFilterType,
    condition_type::ConditionType, data_bar_formatting::DataBarFormatting,
    differential_style_provider::DifferentialStyleProvider, excel_number_format::ExcelNumberFormat,
    font_formatting::FontFormatting, icon_set::IconMultiStateFormatting,
    pattern_formatting::PatternFormatting,
};

/// Represents a description of a conditional formatting rule.
pub trait ConditionalFormattingRule: DifferentialStyleProvider + Debug {
    /// Create a new border formatting structure if it does not exist,
    /// otherwise just return existing object.
    ///
    /// # Returns
    /// Border formatting object, never returns `None`.
    fn create_border_formatting(&mut self) -> &mut dyn BorderFormatting;

    /// Get the border formatting object.
    ///
    /// # Returns
    /// Border formatting object if defined, `None` otherwise.
    fn get_border_formatting(&self) -> Option<&dyn BorderFormatting>;

    /// Create a new font formatting structure if it does not exist,
    /// otherwise just return existing object.
    ///
    /// # Returns
    /// Font formatting object, never returns `None`.
    fn create_font_formatting(&mut self) -> &mut dyn FontFormatting;

    /// Get the font formatting object.
    ///
    /// # Returns
    /// Font formatting object if defined, `None` otherwise.
    fn get_font_formatting(&self) -> Option<&dyn FontFormatting>;

    /// Create a new pattern formatting structure if it does not exist,
    /// otherwise just return existing object.
    ///
    /// # Returns
    /// Pattern formatting object, never returns `None`.
    fn create_pattern_formatting(&mut self) -> &mut dyn PatternFormatting;

    /// Get the pattern formatting object.
    ///
    /// # Returns
    /// Pattern formatting object if defined, `None` otherwise.
    fn get_pattern_formatting(&self) -> Option<&dyn PatternFormatting>;

    /// Get the databar / data-bar formatting object.
    ///
    /// # Returns
    /// Databar formatting object if defined, `None` otherwise.
    fn get_data_bar_formatting(&self) -> Option<&dyn DataBarFormatting>;

    /// Get the icon / multi-state formatting object.
    ///
    /// # Returns
    /// Icon multi-state formatting object if defined, `None` otherwise.
    fn get_multi_state_formatting(&self) -> Option<&dyn IconMultiStateFormatting>;

    /// Get the color scale / color gradient formatting object.
    ///
    /// # Returns
    /// Color scale formatting object if defined, `None` otherwise.
    fn get_color_scale_formatting(&self) -> Option<&dyn ColorScaleFormatting>;

    /// Get the number format defined for this rule.
    ///
    /// # Returns
    /// Number format if defined, or `None` if the cell default should be used.
    fn get_number_format(&self) -> Option<ExcelNumberFormat>;

    /// Get the type of conditional formatting rule.
    ///
    /// # Returns
    /// The type of condition.
    fn get_condition_type(&self) -> ConditionType;

    /// Get the filter type for filter rules.
    ///
    /// This is `None` if `get_condition_type() != ConditionType::FILTER`.
    /// This is always `ConditionFilterType::FILTER` for HSSF rules of type `ConditionType::FILTER`.
    /// For XSSF filter rules, this will indicate the specific type of filter.
    ///
    /// # Returns
    /// Filter type for filter rules, or `None` if not a filter rule.
    fn get_condition_filter_type(&self) -> Option<ConditionFilterType>;

    /// Get the filter configuration data.
    ///
    /// This is `None` if `get_condition_filter_type() == None`.
    /// This means it is always `None` for HSSF, which does not define the extended condition types.
    /// This object contains the additional configuration information for XSSF filter conditions.
    ///
    /// # Returns
    /// The filter configuration data, or `None` if there isn't any.
    fn get_filter_configuration(&self) -> Option<&dyn ConditionFilterData>;

    /// Get the comparison function used when the type of conditional formatting is set to
    /// `ConditionType::CELL_VALUE_IS`.
    ///
    /// MUST be a constant from `ComparisonOperator`.
    ///
    /// # Returns
    /// The conditional format operator.
    fn get_comparison_operation(&self) -> u8;

    /// Get the formula used to evaluate the first operand for the conditional formatting rule.
    ///
    /// If the condition type is `ConditionType::CELL_VALUE_IS`,
    /// this field is the first operand of the comparison.
    /// If type is `ConditionType::FORMULA`, this formula is used
    /// to determine if the conditional formatting is applied.
    ///
    /// If comparison type is `ConditionType::FORMULA` the formula MUST be a Boolean function.
    ///
    /// # Returns
    /// The first formula.
    fn get_formula1(&self) -> Option<&str>;

    /// Get the formula used to evaluate the second operand of the comparison when
    /// comparison type is `ConditionType::CELL_VALUE_IS` and operator
    /// is either `ComparisonOperator::BETWEEN` or `ComparisonOperator::NOT_BETWEEN`.
    ///
    /// # Returns
    /// The second formula.
    fn get_formula2(&self) -> Option<&str>;

    /// Get condition text if it exists.
    ///
    /// XSSF rules store textual condition values as an attribute and also as a formula
    /// that needs shifting. Using the attribute is simpler/faster.
    /// HSSF rules don't have this and return `None`. We can fall back on the formula for those.
    ///
    /// # Returns
    /// Condition text if it exists, or `None`.
    fn get_text(&self) -> Option<&str>;

    /// Get the priority of the rule, if defined, otherwise 0.
    ///
    /// If priority is 0, just use definition order, as that's how older HSSF rules
    /// are evaluated.
    ///
    /// For XSSF, this should always be set. For HSSF, only newer style rules
    /// have this set, older ones will return 0.
    ///
    /// If a rule is created but not yet added to a sheet, this value may not be valid.
    ///
    /// # Returns
    /// Rule priority.
    fn get_priority(&self) -> u32;

    /// Check if conditional formatting rule processing stops when this one is true.
    ///
    /// Always true for HSSF rules, optional flag for XSSF rules.
    /// See Excel help for more.
    ///
    /// # Returns
    /// `true` if conditional formatting rule processing stops when this one is true,
    /// `false` if not.
    fn get_stop_if_true(&self) -> bool;
}
