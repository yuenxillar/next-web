use std::ops::{Deref, DerefMut};

use crate::core::{
    metadata::basic_parameter::BasicParameter, write::handler::write_handler::WriteHandler,
};

#[derive(Default)]
pub struct WriteBasicParameter {
    relative_head_row_index: Option<u32>,
    need_head: Option<bool>,
    custom_write_handler_list: Vec<Box<dyn WriteHandler>>,
    use_default_style: Option<bool>,
    automatic_merge_head: Option<bool>,
    exclude_column_indexes: Option<Vec<u32>>,
    exclude_column_field_names: Option<Vec<Box<str>>>,
    include_column_indexes: Option<Vec<u32>>,
    include_column_field_names: Option<Vec<Box<str>>>,
    /// Data will be order by includeColumnFieldNames or include_column_indexes. default is false.
    order_by_include_column: Option<bool>,

    pub(crate) basic_parameter: BasicParameter,
}

impl WriteBasicParameter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_relative_head_row_index(&self) -> Option<u32> {
        self.relative_head_row_index
    }

    pub fn set_relative_head_row_index(&mut self, relative_head_row_index: u32) {
        self.relative_head_row_index = Some(relative_head_row_index);
    }

    pub fn get_need_head(&self) -> Option<bool> {
        self.need_head
    }

    pub fn set_need_head(&mut self, need_head: bool) {
        self.need_head = Some(need_head);
    }

    pub fn get_custom_write_handler_list(&self) -> &Vec<Box<dyn WriteHandler>> {
        &self.custom_write_handler_list
    }

    pub fn push_custom_write_handler_list<T: WriteHandler>(&mut self, write_handler: T) {
        self.custom_write_handler_list.push(Box::new(write_handler));
    }

    pub fn set_custom_write_handler_list(
        &mut self,
        custom_write_handler_list: Vec<Box<dyn WriteHandler>>,
    ) {
        self.custom_write_handler_list = custom_write_handler_list;
    }

    pub fn get_use_default_style(&self) -> Option<bool> {
        self.use_default_style
    }

    pub fn set_use_default_style(&mut self, use_default_style: bool) {
        self.use_default_style = Some(use_default_style);
    }

    pub fn get_automatic_merge_head(&self) -> Option<bool> {
        self.automatic_merge_head
    }

    pub fn set_automatic_merge_head(&mut self, automatic_merge_head: bool) {
        self.automatic_merge_head = Some(automatic_merge_head);
    }

    pub fn get_exclude_column_indexes(&self) -> Option<&Vec<u32>> {
        self.exclude_column_indexes.as_ref()
    }

    pub fn set_exclude_column_indexes(&mut self, exclude_column_indexes: Vec<u32>) {
        self.exclude_column_indexes = Some(exclude_column_indexes);
    }

    pub fn get_exclude_column_field_names(&self) -> Option<&Vec<Box<str>>> {
        self.exclude_column_field_names.as_ref()
    }

    pub fn set_exclude_column_field_names(&mut self, exclude_column_field_names: Vec<Box<str>>) {
        self.exclude_column_field_names = Some(exclude_column_field_names);
    }

    pub fn get_include_column_indexes(&self) -> Option<&Vec<u32>> {
        self.include_column_indexes.as_ref()
    }

    pub fn set_include_column_indexes(&mut self, include_column_indexes: Vec<u32>) {
        self.include_column_indexes = Some(include_column_indexes);
    }

    pub fn get_include_column_field_names(&self) -> Option<&Vec<Box<str>>> {
        self.include_column_field_names.as_ref()
    }

    pub fn set_include_column_field_names(&mut self, include_column_field_names: Vec<Box<str>>) {
        self.include_column_field_names = Some(include_column_field_names);
    }

    pub fn get_order_by_include_column(&self) -> Option<bool> {
        self.order_by_include_column
    }

    pub fn set_order_by_include_column(&mut self, order_by_include_column: bool) {
        self.order_by_include_column = Some(order_by_include_column);
    }
}

impl Deref for WriteBasicParameter {
    type Target = BasicParameter;

    fn deref(&self) -> &Self::Target {
        &self.basic_parameter
    }
}

impl DerefMut for WriteBasicParameter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.basic_parameter
    }
}
