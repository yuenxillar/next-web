use std::{error::Error, fmt::Display};

use crate::core::metadata::{
    data::cell_data::CellData, property::excel_content_property::ExcelContentProperty,
};

#[derive(Debug, Clone)]
pub struct ExcelDataConvertError<T = ()> {
    message: Option<String>,
    row_index: u32,
    column_index: u32,
    cell_data: CellData<T>,
    excel_content_property: Option<ExcelContentProperty>,
}

impl<T> ExcelDataConvertError<T> {
    pub fn new(
        row_index: u32,
        column_index: u32,
        cell_data: CellData<T>,
        excel_content_property: Option<ExcelContentProperty>,
        message: Option<String>,
    ) -> Self {
        ExcelDataConvertError {
            row_index,
            column_index,
            cell_data,
            excel_content_property,
            message,
        }
    }
}

impl Error for ExcelDataConvertError {}

impl Display for ExcelDataConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExcelDataConvertError: row_index={}, column_index={}, cell_data={:?}, excel_content_property={:?}, message={:?}",
            self.row_index,
            self.column_index,
            self.cell_data,
            self.excel_content_property,
            self.message
        )
    }
}
