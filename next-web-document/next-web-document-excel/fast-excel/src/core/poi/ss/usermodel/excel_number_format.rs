use crate::core::poi::ss::{
    formula::conditional_formatting_evaluator::ConditionalFormattingEvaluator,
    usermodel::{cell::Cell, cell_style::CellStyle},
};

/// Object to hold a number format index and string, for various formatting evaluations
#[derive(Debug, Clone, PartialEq)]
pub struct ExcelNumberFormat {
    idx: i32,
    format: String,
}

impl ExcelNumberFormat {
    /// Creates a new ExcelNumberFormat instance
    /// Use this carefully, prefer factory methods to ensure id/format relationships are not broken or confused.
    /// Left public so `ConditionalFormattingRule::get_number_format()` implementations can use it.
    ///
    /// # Arguments
    /// * `idx` - Excel number format index, either a built-in or a higher custom # mapped in the workbook style table
    /// * `format` - Excel number format string for the index
    pub fn new(idx: i32, format: String) -> Self {
        ExcelNumberFormat { idx, format }
    }

    /// Factory method to create ExcelNumberFormat from cell style
    ///
    /// # Returns
    /// * `None` if the style is null, instance from style data format values otherwise
    pub fn from_style(style: &dyn CellStyle) -> Option<Self> {
        ExcelNumberFormat::new(
            style.get_data_format(),
            style.get_data_format_string().map(ToString::to_string),
        )
    }

    /// Factory method to extract format from cell
    ///
    /// # Arguments
    /// * `cell` - cell to extract format from
    /// * `cf_evaluator` - `ConditionalFormattingEvaluator` to use, or `None` if none in this context
    ///
    /// # Returns
    /// * number format from highest-priority rule with a number format, or the cell style,
    ///   or `None` if none of the above apply/are defined
    pub fn from_cell(
        cell: &dyn Cell,
        cf_evaluator: Option<&ConditionalFormattingEvaluator>,
    ) -> Option<Self> {
        let mut nf = None;

        // First check conditional formatting rules (priority order, per Excel help)
        if let Some(evaluator) = cf_evaluator {
            let rules = evaluator.get_conditional_formatting_for_cell(cell);
            for rule in rules {
                nf = rule.get_number_format();
                if nf.is_some() {
                    break;
                }
            }
        }

        // If no format from conditional formatting, fall back to cell style
        if nf.is_none() {
            let style = cell.get_cell_style();
            if let Some(style) = style {
                nf = ExcelNumberFormat::from_style(style);
            }
        }

        nf
    }

    /// Get the Excel number format index
    ///
    /// # Returns
    /// * Excel number format index, either a built-in or a higher custom # mapped in the workbook style table
    pub fn get_idx(&self) -> i32 {
        self.idx
    }

    /// Get the Excel number format string
    ///
    /// # Returns
    /// * Excel number format string for the index
    pub fn get_format(&self) -> &str {
        &self.format
    }
}
