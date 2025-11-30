use crate::core::{
    metadata::configuration_holder::ConfigurationHolder,
    read::listener::read_listener::ReadListener,
};

pub trait ReadHolder
where
    Self: ConfigurationHolder,
{
    /// What handler does the currently operated cell need to execute
    fn read_listener_list(&self) -> Vec<Box<dyn ReadListener>>;

    fn excel_read_head_property(&self) -> &ExcelReadHeadProperty;
}
