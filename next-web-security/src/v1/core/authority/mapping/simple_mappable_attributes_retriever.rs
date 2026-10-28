use std::collections::HashSet;

use crate::core::authority::mapping::MappableAttributesRetriever;

/// This class implements the [`MappableAttributesRetriever`] trait by just returning a
/// list of mappable attributes as previously set using the corresponding setter method.
///
/// This is a simple in-memory implementation that stores a set of attributes and
/// returns them when requested.
#[derive(Debug, Clone, Default)]
pub struct SimpleMappableAttributesRetriever {
    mappable_attributes: HashSet<String>,
}

impl SimpleMappableAttributesRetriever {
    pub fn set_mappable_attributes(&mut self, attributes: HashSet<String>) {
        self.mappable_attributes = attributes;
    }
}

impl MappableAttributesRetriever for SimpleMappableAttributesRetriever {
    fn get_mappable_attributes(&self) -> &HashSet<String> {
        &self.mappable_attributes
    }
}
