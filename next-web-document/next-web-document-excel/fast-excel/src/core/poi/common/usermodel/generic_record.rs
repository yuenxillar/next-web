use indexmap::IndexMap;
use next_web_core::anys::any_value::AnyValue;

/// Generic record interface (simplified version)
pub trait GenericRecord {
    /// Get generic properties (simplified version)
    fn get_generic_properties(&self) -> Option<IndexMap<String, AnyValue>>;

    /// Get generic children
    fn get_generic_children(&self) -> Option<Vec<&dyn GenericRecord>> {
        None
    }
}
