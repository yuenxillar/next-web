#[cfg(feature = "async")]
use tokio::fs::File;

#[cfg(not(feature = "async"))]
use std::fs::File;
use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::core::{
    metadata::basic_parameter::BasicParameter, support::excel_type::ExcelType,
    write::metadata::write_basic_parameter::WriteBasicParameter,
};

pub struct WriteWorkbook {
    excel_type: Option<ExcelType>,
    file: Option<File>,

    with_bom: Option<bool>,
    template_file: Option<File>,

    password: Option<Box<str>>,
    in_memory: Option<bool>,
    write_excel_on_error: Option<bool>,

    write_basic_parameter: WriteBasicParameter,
}

impl WriteWorkbook {
    pub fn get_excel_type(&self) -> Option<&ExcelType> {
        self.excel_type.as_ref()
    }

    pub fn set_excel_type(&mut self, excel_type: ExcelType) {
        self.excel_type = Some(excel_type);
    }

    pub fn get_file(&self) -> Option<&File> {
        self.file.as_ref()
    }

    pub fn set_file(&mut self, file: Option<File>) {
        self.file = file;
    }

    pub fn get_with_bom(&self) -> Option<bool> {
        self.with_bom
    }

    pub fn set_with_bom(&mut self, with_bom: bool) {
        self.with_bom = Some(with_bom);
    }

    pub fn get_template_file(&self) -> Option<&File> {
        self.template_file.as_ref()
    }

    pub fn set_template_file(&mut self, template_file: File) {
        self.template_file = Some(template_file);
    }

    pub fn get_password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn set_password<T: Into<Box<str>>>(&mut self, password: T) {
        self.password = Some(password.into());
    }

    pub fn get_in_memory(&self) -> Option<bool> {
        self.in_memory
    }

    pub fn set_in_memory(&mut self, in_memory: bool) {
        self.in_memory = Some(in_memory);
    }

    pub fn get_write_excel_on_error(&self) -> Option<bool> {
        self.write_excel_on_error
    }

    pub fn set_write_excel_on_error(&mut self, write_excel_on_error: bool) {
        self.write_excel_on_error = Some(write_excel_on_error);
    }
}

impl Required<WriteBasicParameter> for WriteWorkbook {
    fn get_object(&self) -> &WriteBasicParameter {
        &self.write_basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut WriteBasicParameter {
        &mut self.write_basic_parameter
    }
}

impl Required<BasicParameter> for WriteWorkbook {
    fn get_object(&self) -> &BasicParameter {
        &self.write_basic_parameter.basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut BasicParameter {
        &mut self.write_basic_parameter.basic_parameter
    }
}

impl Deref for WriteWorkbook {
    type Target = WriteBasicParameter;

    fn deref(&self) -> &Self::Target {
        &self.write_basic_parameter
    }
}

impl DerefMut for WriteWorkbook {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.write_basic_parameter
    }
}

impl Default for WriteWorkbook {
    fn default() -> Self {
        Self {
            excel_type: None,
            file: None,
            with_bom: None,
            password: None,
            in_memory: None,
            write_excel_on_error: None,
            write_basic_parameter: Default::default(),
        }
    }
}
