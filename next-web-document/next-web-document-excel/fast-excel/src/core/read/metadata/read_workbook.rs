use std::{collections::HashSet, sync::Arc};

use crate::core::{
    cache::{read_cache::ReadCache, selector::read_cache_selector::ReadCacheSelector},
    enums::{cell_extra_type::CellExtraType, read_default_return::ReadDefaultReturn},
    metadata::basic_parameter::BasicParameter,
    read::metadata::read_basic_parameter::ReadBasicParameter,
    support::excel_type::ExcelType,
};
use next_web_core::traits::required::Required;
#[cfg(feature = "async")]
use tokio::fs::File;
#[cfg(feature = "async")]
use tokio::io::AsyncRead as ReadStream;

#[cfg(not(feature = "async"))]
use std::fs::File;
#[cfg(not(feature = "async"))]
use std::io::Read as ReadStream;

pub struct ReadWorkbook<T> {
    excel_type: Option<ExcelType>,
    ignore_empty_row: Option<bool>,
    reader: Option<Box<dyn ReadStream>>,

    ///  A cache that stores temp data to save memory.
    read_cache: Option<Arc<dyn ReadCache>>,
    read_cache_selector: Option<Box<dyn ReadCacheSelector>>,

    file: Option<File>,
    charset: Option<String>,
    path: Option<String>,

    custom_object: Option<T>,

    // read_cache_selector: Option<ReadCacheSelector>,
    password: Option<String>,
    xlsx_saxparser_factory_name: Option<String>,
    use_default_listener: Option<bool>,
    read_default_return: Option<ReadDefaultReturn>,
    extra_read_set: Option<HashSet<CellExtraType>>,
    num_rows: Option<u32>,
    read_basic_parameter: ReadBasicParameter<T>,
}

impl<T> ReadWorkbook<T> {
    pub fn get_excel_type(&self) -> Option<ExcelType> {
        self.excel_type.clone()
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

    pub fn get_reader(&self) -> Option<&dyn ReadStream> {
        self.reader.as_deref()
    }

    pub fn set_reader(&mut self, reader: Box<dyn ReadStream>) {
        self.reader = Some(reader);
    }

    pub fn get_file(&self) -> Option<&File> {
        self.file.as_ref()
    }

    pub fn set_file(&mut self, file: File) {
        self.file = Some(file);
    }

    pub fn get_custom_object(&self) -> Option<&T> {
        self.custom_object.as_ref()
    }

    pub fn set_custom_object(&mut self, custom_object: T) {
        self.custom_object = Some(custom_object);
    }

    pub fn get_read_cache(&self) -> Option<&Arc<dyn ReadCache>> {
        self.read_cache.as_ref()
    }

    pub fn set_read_cache<R: ReadCache + 'static>(&mut self, read_cache: R) {
        self.read_cache = Some(Arc::new(read_cache));
    }

    pub fn get_read_cache_selector(&self) -> Option<&dyn ReadCacheSelector> {
        self.read_cache_selector.as_deref()
    }

    pub fn set_read_cache_selector<S: ReadCacheSelector + 'static>(
        &mut self,
        read_cache_selector: S,
    ) {
        self.read_cache_selector = Some(Box::new(read_cache_selector));
    }

    pub fn get_charset(&self) -> Option<&str> {
        self.charset.as_deref()
    }

    pub fn set_charset(&mut self, charset: impl ToString) {
        self.charset = Some(charset.to_string());
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

    pub fn get_extra_read_set(&mut self) -> Option<&mut HashSet<CellExtraType>> {
        self.extra_read_set.as_mut()
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
}

impl<T> Required<BasicParameter> for ReadWorkbook<T> {
    fn get_object(&self) -> &BasicParameter {
        &self.read_basic_parameter.basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut BasicParameter {
        &mut self.read_basic_parameter.basic_parameter
    }
}

impl<T> Required<ReadBasicParameter<T>> for ReadWorkbook<T> {
    fn get_object(&self) -> &ReadBasicParameter<T> {
        &self.read_basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut ReadBasicParameter<T> {
        &mut self.read_basic_parameter
    }
}

impl<T> Default for ReadWorkbook<T> {
    fn default() -> Self {
        ReadWorkbook {
            read_basic_parameter: ReadBasicParameter::default(),

            excel_type: None,
            ignore_empty_row: None,
            custom_object: None,
            reader: None,
            read_cache: None,
            read_cache_selector: None,
            file: None,
            charset: None,
            path: None,
            password: None,
            xlsx_saxparser_factory_name: None,
            use_default_listener: None,
            read_default_return: None,
            extra_read_set: None,
            num_rows: None,
        }
    }
}
