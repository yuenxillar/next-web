use std::sync::Arc;

use next_web_core::traits::required::Required;

use crate::core::{
    metadata::{basic_parameter::BasicParameter, parameter_builder::ParameterBuilder},
    read::{
        listener::read_listener::ReadListener, metadata::read_basic_parameter::ReadBasicParameter,
    },
};

pub trait BaseExcelReaderParameterBuilder<T, C>
where
    C: Required<ReadBasicParameter<T>>,
    C: Required<BasicParameter>,
    Self: ParameterBuilder<T, C>,
{
    /// Sets the number of header rows when reading sheet
    ///
    /// - 0: Sheet has no header, first row is data
    /// - 1: Sheet has one row header (default)
    /// - 2: Sheet has two row headers, third row is data
    /// - n: Sheet has n row headers, (n+1)th row is data
    fn head_row_number(&mut self, head_row_number: u32) {
        let paramter: &mut ReadBasicParameter<T> = self.parameter().get_mut_object();

        paramter.set_head_row_number(head_row_number);
    }

    /// Sets whether to use scientific notation for numeric values
    ///
    /// Default is false
    fn use_scientific_format(&mut self, use_scientific_format: bool) {
        let paramter: &mut ReadBasicParameter<T> = self.parameter().get_mut_object();

        paramter.set_use_scientific_format(use_scientific_format);
    }

    /// Registers a custom read listener
    ///
    /// Custom listeners run after default listeners
    fn register_read_listener<L>(&mut self, read_listener: L)
    where
        L: ReadListener<T> + 'static,
    {
        let paramter: &mut ReadBasicParameter<T> = self.parameter().get_mut_object();

        paramter.add_custom_read_listener(Arc::new(read_listener));
    }
}
