use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::core::{
    authority::mapping::{Attributes2GrantedAuthoritiesMapper, MappableAttributesRetriever},
    GrantedAuthority,
};

/// This implements the Attributes2GrantedAuthoritiesMapper and MappableAttributesRetriever interfaces based on the supplied Map.
/// It supports both one-to-one and one-to-many mappings. The granted authorities to map to can be supplied either as a String or as a GrantedAuthority object.
#[derive(Clone)]
pub struct MapBasedAttributes2GrantedAuthoritiesMapper {
    attributes2granted_authorities_map: HashMap<String, Vec<Arc<dyn GrantedAuthority>>>,
    string_separator: Box<str>,
    mappable_attributes: HashSet<String>,
}

impl MapBasedAttributes2GrantedAuthoritiesMapper {
    pub fn get_attributes2granted_authorities_map(
        &self,
    ) -> &HashMap<String, Vec<Arc<dyn GrantedAuthority>>> {
        &self.attributes2granted_authorities_map
    }

    pub fn set_attributes_to_granted_authorities_map(
        &mut self,
        attributes2granted_authorities_map: HashMap<String, Vec<Arc<dyn GrantedAuthority>>>,
    ) {
        assert!(
            !attributes2granted_authorities_map.is_empty(),
            "A non-empty attributes2granted_authorities_map must be supplied"
        );

        self.mappable_attributes = attributes2granted_authorities_map.keys().cloned().collect();
        self.attributes2granted_authorities_map = attributes2granted_authorities_map;
    }

    pub fn get_string_separator(&self) -> &str {
        &self.string_separator
    }

    pub fn set_string_separator(&mut self, string_separator: impl Into<Box<str>>) {
        self.string_separator = string_separator.into();
    }
}

impl Attributes2GrantedAuthoritiesMapper for MapBasedAttributes2GrantedAuthoritiesMapper {
    fn get_granted_authorities(&self, attributes: &[String]) -> Vec<Arc<dyn GrantedAuthority>> {
        let mut result = Vec::<Arc<dyn GrantedAuthority>>::new();
        attributes
            .iter()
            .filter_map(|attribute| self.attributes2granted_authorities_map.get(attribute))
            .for_each(|granted| result.extend(granted.iter().cloned()));

        result
    }
}

impl MappableAttributesRetriever for MapBasedAttributes2GrantedAuthoritiesMapper {
    fn get_mappable_attributes(&self) -> &HashSet<String> {
        &self.mappable_attributes
    }
}

impl Default for MapBasedAttributes2GrantedAuthoritiesMapper {
    fn default() -> Self {
        Self {
            attributes2granted_authorities_map: HashMap::new(),
            mappable_attributes: HashSet::new(),
            string_separator: ",".into(),
        }
    }
}
