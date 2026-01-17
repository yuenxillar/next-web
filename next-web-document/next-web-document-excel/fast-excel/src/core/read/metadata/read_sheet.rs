use std::ops::{Deref, DerefMut};

use next_web_core::traits::required::Required;

use crate::core::{
    metadata::basic_parameter::BasicParameter,
    read::metadata::read_basic_parameter::ReadBasicParameter,
};

/// Read sheet
#[derive(Clone)]
pub struct ReadSheet<T> {
    /// Starting from 0
    sheet_no: Option<u32>,
    /// sheet name
    sheet_name: Option<String>,
    /// The number of rows to read, the default is all, start with 0.
    num_rows: Option<u32>,

    base: ReadBasicParameter<T>,
}

impl<T> ReadSheet<T> {
    pub fn new(sheet_no: u32, sheet_name: String, num_rows: u32) -> Self {
        Self {
            sheet_no: Some(sheet_no),
            sheet_name: Some(sheet_name),
            num_rows: Some(num_rows),

            base: Default::default(),
        }
    }

    pub fn with_sheet_no(sheet_no: u32) -> Self {
        let mut read_sheet = Self::default();
        read_sheet.set_sheet_no(sheet_no);

        read_sheet
    }

    pub fn with_sheet_no_and_sheet_name(sheet_no: u32, sheet_name: impl ToString) -> Self {
        let mut read_sheet = Self::with_sheet_no(sheet_no);
        read_sheet.set_sheet_name(sheet_name.to_string());

        read_sheet
    }

    pub fn get_sheet_no(&self) -> Option<u32> {
        self.sheet_no
    }

    pub fn get_sheet_name(&self) -> Option<&str> {
        self.sheet_name.as_deref()
    }

    pub fn get_num_rows(&self) -> Option<u32> {
        self.num_rows
    }

    pub fn set_sheet_no(&mut self, sheet_no: u32) {
        self.sheet_no = Some(sheet_no);
    }

    pub fn set_sheet_name(&mut self, sheet_name: String) {
        self.sheet_name = Some(sheet_name);
    }

    pub fn set_num_rows(&mut self, num_rows: u32) {
        self.num_rows = Some(num_rows);
    }

    pub fn copy_basic_parameter(&mut self, other: ReadSheet<T>) {
        if let Some(head_row_number) = other.get_head_row_number() {
            self.set_head_row_number(head_row_number);
        }

        self.set_custom_read_listener(other.get_custom_read_listener_list());

        if let Some(head) = other.get_head() {
            self.set_head(head.clone());
        }

        if let Some(custom_converter_list) = other.get_custom_converter_list() {
            self.set_custom_converter_list(custom_converter_list.clone());
        }

        if let Some(auto_trim) = other.get_auto_trim() {
            self.set_auto_trim(auto_trim);
        }

        if let Some(used) = other.get_use1904windowing() {
            self.set_use1904windowing(used);
        }

        if let Some(num_rows) = other.get_num_rows() {
            self.set_num_rows(num_rows);
        }
    }
}

impl<T> Required<BasicParameter> for ReadSheet<T> {
    fn get_object(&self) -> &BasicParameter {
        &self.base.basic_parameter
    }

    fn get_mut_object(&mut self) -> &mut BasicParameter {
        &mut self.base.basic_parameter
    }
}

impl<T> Required<ReadBasicParameter<T>> for ReadSheet<T> {
    fn get_object(&self) -> &ReadBasicParameter<T> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut ReadBasicParameter<T> {
        &mut self.base
    }
}

impl<T> Deref for ReadSheet<T> {
    type Target = ReadBasicParameter<T>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T> DerefMut for ReadSheet<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<T> Default for ReadSheet<T> {
    fn default() -> Self {
        Self {
            sheet_no: None,
            sheet_name: None,
            num_rows: None,
            base: ReadBasicParameter::<T>::default(),
        }
    }
}
