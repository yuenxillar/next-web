use std::collections::HashSet;

use crate::core::{
    enums::{cell_extra_type::CellExtraType, read_default_return::ReadDefaultReturn},
    read::metadata::read_basic_parameter::ReadBasicParameter,
    support::excel_type::ExcelType,
};
#[cfg(feature = "async")]
use tokio::fs::File;

#[cfg(not(feature = "async"))]
use std::fs::File;

pub struct ReadWorkbook {
    excel_type: Option<ExcelType>,
    ignore_empty_row: Option<bool>,
    file: Option<File>,
    path: Option<String>,

    // read_cache_selector: Option<ReadCacheSelector>,
    password: Option<String>,
    xlsx_saxparser_factory_name: Option<String>,
    use_default_listener: Option<bool>,
    read_default_return: Option<ReadDefaultReturn>,
    extra_read_set: Option<HashSet<CellExtraType>>,
    num_rows: Option<u32>,
    read_basic_parameter: Option<ReadBasicParameter>,
}

impl ReadWorkbook {
    pub fn get_excel_type(&self) -> Option<&ExcelType> {
        self.excel_type.as_ref()
    }

    pub fn set_excel_type(&mut self, excel_type: ExcelType) {
        self.excel_type = Some(excel_type);
    }

    pub fn get_ignore_empty_row(&self) -> Option<bool> {
        self.ignore_empty_row
    }

    pub fn set_ignore_empty_row(&mut self, ignore_empty_row: bool) {
        self.ignore_empty_row = Some(ignore_empty_row);
    }

    pub fn get_file(&self) -> Option<&File> {
        self.file.as_ref()
    }

    pub fn set_file(&mut self, file: File) {
        self.file = Some(file);
    }

    pub fn get_path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn set_path(&mut self, path: impl ToString) {
        self.path = Some(path.to_string());
    }

    pub fn get_password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn set_password(&mut self, password: impl ToString) {
        self.password = Some(password.to_string());
    }

    pub fn get_xlsx_saxparser_factory_name(&self) -> Option<String> {
        self.xlsx_saxparser_factory_name.clone()
    }

    pub fn set_xlsx_saxparser_factory_name(&mut self, xlsx_saxparser_factory_name: impl ToString) {
        self.xlsx_saxparser_factory_name = Some(xlsx_saxparser_factory_name.to_string());
    }

    pub fn get_use_default_listener(&self) -> Option<bool> {
        self.use_default_listener
    }

    pub fn set_use_default_listener(&mut self, use_default_listener: bool) {
        self.use_default_listener = Some(use_default_listener);
    }

    pub fn get_read_default_return(&self) -> Option<ReadDefaultReturn> {
        self.read_default_return
    }

    pub fn set_read_default_return(&mut self, read_default_return: ReadDefaultReturn) {
        self.read_default_return = Some(read_default_return);
    }

    pub fn get_extra_read_set(&self) -> Option<HashSet<CellExtraType>> {
        self.extra_read_set.clone()
    }

    pub fn set_extra_read_set(&mut self, extra_read_set: HashSet<CellExtraType>) {
        self.extra_read_set = Some(extra_read_set);
    }

    pub fn get_num_rows(&self) -> Option<u32> {
        self.num_rows
    }

    pub fn set_num_rows(&mut self, num_rows: u32) {
        self.num_rows = Some(num_rows);
    }

    pub fn get_read_basic_parameter(&self) -> Option<&ReadBasicParameter> {
        self.read_basic_parameter.as_ref()
    }

    pub fn set_read_basic_parameter(&mut self, read_basic_parameter: ReadBasicParameter) {
        self.read_basic_parameter = Some(read_basic_parameter);
    }
}
