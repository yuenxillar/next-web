use std::{collections::HashSet, sync::Arc};

use crate::core::{
    authority::{mapping::GrantedAuthoritiesMapper, SimpleGrantedAuthority},
    GrantedAuthority,
};

/// Simple one-to-one GrantedAuthoritiesMapper which allows for case conversion of the authority
/// name and the addition of a string prefix (which defaults to ROLE_ ).
#[derive(Clone)]
pub struct SimpleAuthorityMapper {
    default_authority: Option<Arc<dyn GrantedAuthority>>,
    prefix: String,
    convert_to_uppercase: bool,
    convert_to_lowercase: bool,
}

impl SimpleAuthorityMapper {
    /// Sets the prefix which should be added to the authority name (if it doesn't already exist)
    pub fn set_prefix(&mut self, prefix: impl Into<String>) {
        self.prefix = prefix.into();
    }

    /// Whether to convert the authority value to upper case in the mapping.
    pub fn set_convert_to_uppercase(&mut self, value: bool) {
        self.convert_to_uppercase = value;
    }

    /// Whether to convert the authority value to lower case in the mapping.
    pub fn set_convert_to_lowercase(&mut self, value: bool) {
        self.convert_to_lowercase = value;
    }

    /// Sets a default authority to be assigned to all users
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

    fn map_authority(&self, name: &str) -> Arc<dyn GrantedAuthority> {
        let mut name = name.to_string();

        if self.convert_to_uppercase {
            name = name.to_uppercase();
        } else if self.convert_to_lowercase {
            name = name.to_lowercase();
        }
        if !self.prefix.is_empty() && !name.starts_with(&self.prefix) {
            name = format!("{}{}", self.prefix, name);
        }

        Arc::new(SimpleGrantedAuthority::new(name))
    }
}

impl GrantedAuthoritiesMapper for SimpleAuthorityMapper {
    fn map_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        let mut mapped = HashSet::<Arc<dyn GrantedAuthority>>::with_capacity(authorities.len());

        authorities
            .iter()
            .filter_map(|auth| auth.authority())
            .for_each(|authority| {
                mapped.insert(self.map_authority(authority));
            });

        if let Some(authority) = self
            .default_authority
            .as_ref()
            .map(|authority| authority.clone())
        {
            mapped.insert(authority);
        }

        mapped.into_iter().collect()
    }
}

impl Default for SimpleAuthorityMapper {
    fn default() -> Self {
        Self {
            default_authority: None,
            prefix: String::from("ROLE_"),
            convert_to_uppercase: false,
            convert_to_lowercase: false,
        }
    }
}
