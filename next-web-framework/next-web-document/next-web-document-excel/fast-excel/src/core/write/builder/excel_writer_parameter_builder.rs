use next_web_core::traits::required::Required;
use std::ops::DerefMut;

use crate::core::{
    metadata::{basic_parameter::BasicParameter, parameter_builder::ParameterBuilder},
    write::{
        handler::write_handler::WriteHandler, metadata::write_basic_parameter::WriteBasicParameter,
    },
};

pub trait ExcelWriterParameterBuilder<C>
where
    C: Required<WriteBasicParameter>,
    C: Required<BasicParameter>,
    Self: ParameterBuilder<C>,
{
    fn relative_head_row_index(&mut self, relative_head_row_index: u32) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_relative_head_row_index(relative_head_row_index);

        self
    }

    fn need_head(&mut self, need_head: bool) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_need_head(need_head);
        self
    }

    fn register_write_handler<T: WriteHandler>(&mut self, write_handler: T) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.push_custom_write_handler_list(write_handler);
        self
    }

    fn use_default_style(&mut self, use_default_style: bool) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_use_default_style(use_default_style);
        self
    }

    fn automatic_merge_head(&mut self, automatic_merge_head: bool) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_automatic_merge_head(automatic_merge_head);
        self
    }

    fn exclude_column_indexes(&mut self, exclude_column_indexes: Vec<u32>) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_exclude_column_indexes(exclude_column_indexes);
        self
    }

    fn exclude_column_filed_names(
        &mut self,
        exclude_column_filed_names: Vec<Box<str>>,
    ) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_exclude_column_field_names(exclude_column_filed_names);
        self
    }

    fn include_column_indexes(&mut self, include_column_indexes: Vec<u32>) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_include_column_indexes(include_column_indexes);
        self
    }

    fn include_column_field_names(
        &mut self,
        include_column_field_names: Vec<Box<str>>,
    ) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_include_column_field_names(include_column_field_names);
        self
    }

    fn order_by_include_column(&mut self, order_by_include_column: bool) -> &mut Self {
        let parameter: &mut WriteBasicParameter = self.parameter().get_mut_object();
        parameter.set_order_by_include_column(order_by_include_column);
        self
    }
}
