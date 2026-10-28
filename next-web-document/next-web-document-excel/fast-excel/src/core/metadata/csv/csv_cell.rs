use bigdecimal::BigDecimal;
use chrono::{DateTime, Local};

use crate::core::{
    enums::{cell_data::CellType, numeric_cell_type::NumericCellType},
    metadata::{
        csv::{csv_row::CsvRow, csv_sheet::CsvSheet, csv_workbook::CsvWorkbook},
        data::formula_data::FormulaData,
    },
    poi::ss::usermodel::{cell_style::CellStyle, rich_text_string::RichTextString},
};

#[derive(Debug, Clone)]
pub struct CsvCell {
    column_index: Option<u32>,
    cell_type: Option<CellType>,
    numeric_cell_type: Option<NumericCellType>,
    csv_workbook: Option<CsvWorkbook>,
    csv_sheet: Option<CsvSheet>,
    csv_row: Option<CsvRow>,

    number_value: Option<BigDecimal>,
    string_value: Option<String>,
    date_value: Option<DateTime<Local>>,

    formula_data: Option<FormulaData>,
    rich_text_string: Option<Box<dyn RichTextString>>,
    cell_style: Option<Box<dyn CellStyle>>,
}
