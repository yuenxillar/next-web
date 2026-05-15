use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use crate::core::{
    granted_authority::GrantedAuthority, simple_granted_authority::SimpleGrantedAuthority,
};

pub trait GrantedAuthoritiesMapper: Send + Sync {
    fn map_authorities(
        &self,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Vec<Arc<dyn GrantedAuthority>>;
}

#[derive(Clone, Default)]
pub struct NullAuthoritiesMapper;

impl GrantedAuthoritiesMapper for NullAuthoritiesMapper {
    fn map_authorities(
        &self,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        authorities
    }
}

pub trait Attributes2GrantedAuthoritiesMapper: Send + Sync {
    fn get_granted_authorities(
        &self,
        attributes: Vec<String>,
    ) -> Vec<Arc<dyn GrantedAuthority>>;
}

pub trait MappableAttributesRetriever: Send + Sync {
    fn get_mappable_attributes(&self) -> Vec<String>;
}

#[derive(Clone, Default)]
pub struct SimpleAuthorityMapper {
    default_authority: Option<Arc<dyn GrantedAuthority>>,
    prefix: String,
    convert_to_uppercase: bool,
    convert_to_lowercase: bool,
}

impl SimpleAuthorityMapper {
    pub fn new() -> Self {
        Self {
            default_authority: None,
            prefix: String::from("ROLE_"),
            convert_to_uppercase: false,
            convert_to_lowercase: false,
        }
    }

    pub fn set_prefix(&mut self, prefix: impl Into<String>) {
        self.prefix = prefix.into();
    }

    pub fn set_convert_to_uppercase(&mut self, value: bool) {
        self.convert_to_uppercase = value;
    }

    pub fn set_convert_to_lowercase(&mut self, value: bool) {
        self.convert_to_lowercase = value;
    }

    pub fn set_default_authority(&mut self, authority: impl Into<String>) {
        let authority = authority.into();
        assert!(
            !authority.trim().is_empty(),
            "The authority name cannot be set to an empty value"
        );
        self.default_authority = Some(Arc::new(SimpleGrantedAuthority::new(authority)));
    }

    pub fn after_properties_set(&self) {
        assert!(
            !(self.convert_to_uppercase && self.convert_to_lowercase),
            "Either convertToUpperCase or convertToLowerCase can be set to true, but not both"
        );
    }

    fn map_authority_name(&self, mut name: String) -> String {
        if self.convert_to_uppercase {
            name = name.to_uppercase();
        } else if self.convert_to_lowercase {
            name = name.to_lowercase();
        }
        if !self.prefix.is_empty() && !name.starts_with(&self.prefix) {
            name = format!("{}{}", self.prefix, name);
        }
        name
    }
}

impl GrantedAuthoritiesMapper for SimpleAuthorityMapper {
    fn map_authorities(
        &self,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        self.after_properties_set();

        let mut mapped = BTreeMap::<String, Arc<dyn GrantedAuthority>>::new();
        for authority in authorities {
            if let Some(name) = block_on(authority.get_authority()) {
                let mapped_name = self.map_authority_name(name);
                mapped
                    .entry(mapped_name.clone())
                    .or_insert_with(|| Arc::new(SimpleGrantedAuthority::new(mapped_name)));
            }
        }

        if let Some(default_authority) = &self.default_authority {
            if let Some(name) = block_on(default_authority.get_authority()) {
                mapped
                    .entry(name.clone())
                    .or_insert_with(|| default_authority.clone());
            }
        }

        mapped.into_values().collect()
    }
}

#[derive(Clone, Default)]
pub struct SimpleAttributes2GrantedAuthoritiesMapper {
    attribute_prefix: String,
    convert_attribute_to_uppercase: bool,
    convert_attribute_to_lowercase: bool,
    add_prefix_if_already_existing: bool,
}

impl SimpleAttributes2GrantedAuthoritiesMapper {
    pub fn new() -> Self {
        Self {
            attribute_prefix: String::from("ROLE_"),
            convert_attribute_to_uppercase: false,
            convert_attribute_to_lowercase: false,
            add_prefix_if_already_existing: false,
        }
    }

    pub fn set_convert_attribute_to_lowercase(&mut self, value: bool) {
        self.convert_attribute_to_lowercase = value;
    }

    pub fn set_convert_attribute_to_uppercase(&mut self, value: bool) {
        self.convert_attribute_to_uppercase = value;
    }

    pub fn set_attribute_prefix(&mut self, prefix: impl Into<String>) {
        self.attribute_prefix = prefix.into();
    }

    pub fn set_add_prefix_if_already_existing(&mut self, value: bool) {
        self.add_prefix_if_already_existing = value;
    }

    pub fn after_properties_set(&self) {
        assert!(
            !(self.convert_attribute_to_uppercase && self.convert_attribute_to_lowercase),
            "Either convertAttributeToUpperCase or convertAttributeToLowerCase can be set to true, but not both"
        );
    }

    fn attribute_prefix(&self) -> &str {
        &self.attribute_prefix
    }

    fn get_granted_authority(&self, mut attribute: String) -> Arc<dyn GrantedAuthority> {
        if self.convert_attribute_to_lowercase {
            attribute = attribute.to_lowercase();
        } else if self.convert_attribute_to_uppercase {
            attribute = attribute.to_uppercase();
        }

        let authority = if self.add_prefix_if_already_existing
            || !attribute.starts_with(self.attribute_prefix())
        {
            format!("{}{}", self.attribute_prefix(), attribute)
        } else {
            attribute
        };
        Arc::new(SimpleGrantedAuthority::new(authority))
    }
}

impl Attributes2GrantedAuthoritiesMapper for SimpleAttributes2GrantedAuthoritiesMapper {
    fn get_granted_authorities(
        &self,
        attributes: Vec<String>,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        self.after_properties_set();
        attributes
            .into_iter()
            .map(|attribute| self.get_granted_authority(attribute))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub enum GrantedAuthorityValue {
    Authority(String),
    Authorities(Vec<GrantedAuthorityValue>),
}

impl From<String> for GrantedAuthorityValue {
    fn from(value: String) -> Self {
        Self::Authority(value)
    }
}

impl From<&str> for GrantedAuthorityValue {
    fn from(value: &str) -> Self {
        Self::Authority(value.to_string())
    }
}

impl From<Vec<String>> for GrantedAuthorityValue {
    fn from(value: Vec<String>) -> Self {
        Self::Authorities(value.into_iter().map(GrantedAuthorityValue::from).collect())
    }
}

impl From<Vec<&str>> for GrantedAuthorityValue {
    fn from(value: Vec<&str>) -> Self {
        Self::Authorities(value.into_iter().map(GrantedAuthorityValue::from).collect())
    }
}

impl From<Vec<GrantedAuthorityValue>> for GrantedAuthorityValue {
    fn from(value: Vec<GrantedAuthorityValue>) -> Self {
        Self::Authorities(value)
    }
}

#[derive(Clone, Default)]
pub struct MapBasedAttributes2GrantedAuthoritiesMapper {
    attributes_to_granted_authorities_map:
        HashMap<String, Vec<Arc<dyn GrantedAuthority>>>,
    string_separator: String,
    mappable_attributes: BTreeSet<String>,
}

impl MapBasedAttributes2GrantedAuthoritiesMapper {
    pub fn new() -> Self {
        Self {
            attributes_to_granted_authorities_map: HashMap::new(),
            string_separator: String::from(","),
            mappable_attributes: BTreeSet::new(),
        }
    }

    pub fn after_properties_set(&self) {
        assert!(
            !self.attributes_to_granted_authorities_map.is_empty(),
            "attributes2grantedAuthoritiesMap must be set"
        );
    }

    pub fn set_string_separator(&mut self, string_separator: impl Into<String>) {
        self.string_separator = string_separator.into();
    }

    pub fn set_attributes_to_granted_authorities_map(
        &mut self,
        attributes_to_granted_authorities_map: HashMap<String, GrantedAuthorityValue>,
    ) {
        assert!(
            !attributes_to_granted_authorities_map.is_empty(),
            "A non-empty attributes2grantedAuthoritiesMap must be supplied"
        );

        let processed = attributes_to_granted_authorities_map
            .into_iter()
            .map(|(key, value)| (key, self.get_granted_authority_collection(value)))
            .collect::<HashMap<_, _>>();

        self.mappable_attributes = processed.keys().cloned().collect();
        self.attributes_to_granted_authorities_map = processed;
    }

    fn get_granted_authority_collection(
        &self,
        value: GrantedAuthorityValue,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        let mut result = Vec::new();
        self.add_granted_authority_collection(&mut result, value);
        result
    }

    fn add_granted_authority_collection(
        &self,
        result: &mut Vec<Arc<dyn GrantedAuthority>>,
        value: GrantedAuthorityValue,
    ) {
        match value {
            GrantedAuthorityValue::Authority(value) => {
                for token in value.split(&self.string_separator) {
                    let token = token.trim();
                    if !token.is_empty() {
                        result.push(Arc::new(SimpleGrantedAuthority::new(token)));
                    }
                }
            }
            GrantedAuthorityValue::Authorities(values) => {
                for value in values {
                    self.add_granted_authority_collection(result, value);
                }
            }
        }
    }
}

impl Attributes2GrantedAuthoritiesMapper for MapBasedAttributes2GrantedAuthoritiesMapper {
    fn get_granted_authorities(
        &self,
        attributes: Vec<String>,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        let mut result = Vec::new();
        for attribute in attributes {
            if let Some(granted) = self.attributes_to_granted_authorities_map.get(&attribute) {
                result.extend(granted.iter().cloned());
            }
        }
        result
    }
}

impl MappableAttributesRetriever for MapBasedAttributes2GrantedAuthoritiesMapper {
    fn get_mappable_attributes(&self) -> Vec<String> {
        self.mappable_attributes.iter().cloned().collect()
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    futures::executor::block_on(future)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::core::{
        authority_mapping::{
            Attributes2GrantedAuthoritiesMapper, GrantedAuthoritiesMapper,
            GrantedAuthorityValue, MapBasedAttributes2GrantedAuthoritiesMapper,
            MappableAttributesRetriever, SimpleAttributes2GrantedAuthoritiesMapper,
            SimpleAuthorityMapper,
        },
        authority_utils::AuthorityUtils,
    };

    #[test]
    fn simple_authority_mapper_adds_prefix_and_case_normalizes() {
        let mut mapper = SimpleAuthorityMapper::new();
        mapper.set_convert_to_uppercase(true);

        let mapped = mapper.map_authorities(AuthorityUtils::create_authority_list(["user"]));
        let names = mapped
            .into_iter()
            .map(|authority| super::block_on(authority.get_authority()))
            .collect::<Vec<_>>();

        assert_eq!(names.len(), 1);
        assert_eq!(names.into_iter().next().unwrap(), Some(String::from("ROLE_USER")));
    }

    #[test]
    fn simple_attributes_mapper_maps_roles_one_to_one() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::new();
        mapper.set_convert_attribute_to_uppercase(true);

        let mapped =
            mapper.get_granted_authorities(vec![String::from("user"), String::from("role_admin")]);
        let names = mapped
            .into_iter()
            .map(|authority| super::block_on(authority.get_authority()).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(names, vec![String::from("ROLE_USER"), String::from("ROLE_ADMIN")]);
    }

    #[test]
    fn map_based_attributes_mapper_supports_one_to_many_values() {
        let mut mapper = MapBasedAttributes2GrantedAuthoritiesMapper::new();
        mapper.set_attributes_to_granted_authorities_map(HashMap::from([
            (
                String::from("group-a"),
                GrantedAuthorityValue::Authorities(vec![
                    GrantedAuthorityValue::from("ROLE_USER"),
                    GrantedAuthorityValue::from("ROLE_AUDIT,ROLE_REPORT"),
                ]),
            ),
            (
                String::from("group-b"),
                GrantedAuthorityValue::from(vec!["ROLE_ADMIN"]),
            ),
        ]));

        let mapped = mapper.get_granted_authorities(vec![
            String::from("group-a"),
            String::from("group-b"),
        ]);
        let names = mapped
            .into_iter()
            .map(|authority| super::block_on(authority.get_authority()).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                String::from("ROLE_USER"),
                String::from("ROLE_AUDIT"),
                String::from("ROLE_REPORT"),
                String::from("ROLE_ADMIN"),
            ]
        );
        assert_eq!(
            mapper.get_mappable_attributes(),
            vec![String::from("group-a"), String::from("group-b")]
        );
    }
}
