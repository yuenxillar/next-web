use crate::core::{
    metadata::configuration_holder::ConfigurationHolder,
    read::{
        listener::read_listener::ReadListener,
        metadata::property::excel_read_head_property::ExcelReadHeadProperty,
    },
};

pub trait ReadHolder<T>
where
    Self: ConfigurationHolder,
{
    /// What handler does the currently operated cell need to execute
    fn read_listener_list(&self) -> Vec<&dyn ReadListener<T>>;

    fn excel_read_head_property(&self) -> &ExcelReadHeadProperty;
}
