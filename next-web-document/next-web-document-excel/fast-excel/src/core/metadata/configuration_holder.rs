use std::collections::HashMap;

use crate::core::{
    converters::{converter::Converter, converter_key_build::ConverterKey},
    metadata::{global_configuration::GlobalConfiguration, holder::Holder},
};

pub trait ConfigurationHolder
where
    Self: Holder,
{
    /// Record whether it's new or from cache
    fn is_new(&self) -> bool;

    /// Some global variables
    fn global_configuration(&self) -> &GlobalConfiguration;

    /// What converter does the currently operated cell need to execute
    fn converter_map(&self) -> &HashMap<ConverterKey, Box<dyn Converter>>;
}
