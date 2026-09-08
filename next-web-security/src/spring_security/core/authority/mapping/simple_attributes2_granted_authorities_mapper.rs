use std::sync::Arc;

use crate::core::authority::mapping::Attributes2GrantedAuthoritiesMapper;
use crate::core::authority::SimpleGrantedAuthority;
use crate::core::GrantedAuthority;

/// This class implements the [`Attributes2GrantedAuthoritiesMapper`] trait by doing a
/// one-to-one mapping from roles to Spring Security `GrantedAuthority`s. Optionally a
/// prefix can be added, and the attribute name can be converted to upper or lower case.
///
/// By default, the attribute is prefixed with "ROLE_" unless it already starts with
/// "ROLE_", and no case conversion is done.
///
/// # Example
/// ```
/// use your_crate::core::authority::mapping::SimpleAttributes2GrantedAuthoritiesMapper;
///
/// let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::new();
/// mapper.set_attribute_prefix("PERM_");
/// mapper.set_convert_attribute_to_upper_case(true);
/// mapper.after_properties_set().unwrap();
///
/// let authorities = mapper.get_granted_authorities(&[
///     "read".to_string(),
///     "write".to_string(),
/// ]);
///
/// assert_eq!(authorities.len(), 2);
/// assert_eq!(authorities[0].authority(), Some("PERM_READ"));
/// assert_eq!(authorities[1].authority(), Some("PERM_WRITE"));
/// ```
#[derive(Debug, Clone)]
pub struct SimpleAttributes2GrantedAuthoritiesMapper {
    attribute_prefix: String,
    convert_attribute_to_upper_case: bool,
    convert_attribute_to_lower_case: bool,
    add_prefix_if_already_existing: bool,
}

impl SimpleAttributes2GrantedAuthoritiesMapper {
    /// Check whether all properties have been set to correct values.
    ///
    /// # Returns
    /// `Ok(())` if configuration is valid.
    ///
    /// # Errors
    /// Returns an error if both `convert_attribute_to_upper_case` and
    /// `convert_attribute_to_lower_case` are set to `true`.
    pub fn after_properties_set(&self) -> Result<(), &'static str> {
        if self.convert_attribute_to_upper_case && self.convert_attribute_to_lower_case {
            return Err(
                "Either convert_attribute_to_upper_case or convert_attribute_to_lower_case \
                 can be set to true, but not both",
            );
        }
        Ok(())
    }

    /// Sets the prefix to be added to attributes.
    ///
    /// # Arguments
    /// * `prefix` - The prefix to use (e.g., "ROLE_", "PERM_")
    pub fn set_attribute_prefix(&mut self, prefix: impl Into<String>) {
        self.attribute_prefix = prefix.into();
    }

    /// Sets whether to convert attribute names to uppercase.
    ///
    /// # Arguments
    /// * `enable` - `true` to convert to uppercase, `false` otherwise
    ///
    /// # Note
    /// Cannot be enabled together with `set_convert_attribute_to_lower_case`.
    pub fn set_convert_attribute_to_upper_case(&mut self, enable: bool) {
        self.convert_attribute_to_upper_case = enable;
    }

    /// Sets whether to convert attribute names to lowercase.
    ///
    /// # Arguments
    /// * `enable` - `true` to convert to lowercase, `false` otherwise
    ///
    /// # Note
    /// Cannot be enabled together with `set_convert_attribute_to_upper_case`.
    pub fn set_convert_attribute_to_lower_case(&mut self, enable: bool) {
        self.convert_attribute_to_lower_case = enable;
    }

    /// Sets whether to add the prefix even if the attribute already starts with it.
    ///
    /// # Arguments
    /// * `enable` - `true` to always add prefix, `false` to only add if not already present
    ///
    /// # Example
    /// ```
    /// # use your_crate::core::authority::mapping::SimpleAttributes2GrantedAuthoritiesMapper;
    ///
    /// let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::new();
    /// mapper.set_add_prefix_if_already_existing(true);
    ///
    /// let authorities = mapper.get_granted_authorities(&[
    ///     "ROLE_ADMIN".to_string(),
    ///     "USER".to_string(),
    /// ]);
    ///
    /// // "ROLE_ADMIN" becomes "ROLE_ROLE_ADMIN" because add_prefix_if_already_existing is true
    /// assert_eq!(authorities[0].authority(), Some("ROLE_ROLE_ADMIN"));
    /// assert_eq!(authorities[1].authority(), Some("ROLE_USER"));
    /// ```
    pub fn set_add_prefix_if_already_existing(&mut self, enable: bool) {
        self.add_prefix_if_already_existing = enable;
    }

    /// Gets the current attribute prefix.
    pub fn attribute_prefix(&self) -> &str {
        &self.attribute_prefix
    }

    /// Gets whether uppercase conversion is enabled.
    pub fn is_convert_attribute_to_upper_case(&self) -> bool {
        self.convert_attribute_to_upper_case
    }

    /// Gets whether lowercase conversion is enabled.
    pub fn is_convert_attribute_to_lower_case(&self) -> bool {
        self.convert_attribute_to_lower_case
    }

    /// Gets whether prefix is always added.
    pub fn is_add_prefix_if_already_existing(&self) -> bool {
        self.add_prefix_if_already_existing
    }

    /// Map the given attribute one-to-one to a `GrantedAuthority`.
    ///
    /// This method applies case conversion (if configured) and prefix addition
    /// (if configured) to the attribute.
    ///
    /// # Arguments
    /// * `attribute` - The attribute to map
    ///
    /// # Returns
    /// A `GrantedAuthority` representing the mapped attribute.
    fn map_attribute(&self, attribute: &str) -> SimpleGrantedAuthority {
        let mut attribute = attribute.to_string();
        // Apply case conversion
        if self.is_convert_attribute_to_lower_case() {
            attribute = attribute.to_lowercase();
        } else if self.is_convert_attribute_to_upper_case() {
            attribute = attribute.to_uppercase();
        }

        // Apply prefix
        if self.is_add_prefix_if_already_existing()
            || !attribute.starts_with(&self.attribute_prefix)
        {
            SimpleGrantedAuthority::new(format!("{}{}", self.attribute_prefix, attribute))
        } else {
            SimpleGrantedAuthority::new(attribute)
        }
    }
}

impl Attributes2GrantedAuthoritiesMapper for SimpleAttributes2GrantedAuthoritiesMapper {
    /// Map the given list of string attributes one-to-one to `GrantedAuthority`s.
    ///
    /// # Arguments
    /// * `attributes` - The attributes to map
    ///
    /// # Returns
    /// A vector of `GrantedAuthority`s mapped from the attributes.
    ///
    /// # Panics
    /// Panics if the mapper is not properly configured (both case conversion flags set).
    fn get_granted_authorities(&self, attributes: &[String]) -> Vec<Arc<dyn GrantedAuthority>> {
        attributes
            .iter()
            .map(|attr| Arc::new(self.map_attribute(attr.as_str())) as Arc<dyn GrantedAuthority>)
            .collect()
    }
}

impl Default for SimpleAttributes2GrantedAuthoritiesMapper {
    fn default() -> Self {
        Self {
            attribute_prefix: "ROLE_".to_string(),
            convert_attribute_to_upper_case: false,
            convert_attribute_to_lower_case: false,
            add_prefix_if_already_existing: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mapper() {
        let mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        let authorities =
            mapper.get_granted_authorities(&["ADMIN".to_string(), "USER".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("ROLE_ADMIN"));
        assert_eq!(authorities[1].authority(), Some("ROLE_USER"));
    }

    #[test]
    fn test_mapper_with_prefix() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_attribute_prefix("PERM_");

        let authorities =
            mapper.get_granted_authorities(&["READ".to_string(), "WRITE".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("PERM_READ"));
        assert_eq!(authorities[1].authority(), Some("PERM_WRITE"));
    }

    #[test]
    fn test_mapper_with_upper_case_conversion() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_convert_attribute_to_upper_case(true);

        let authorities =
            mapper.get_granted_authorities(&["admin".to_string(), "user".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("ROLE_ADMIN"));
        assert_eq!(authorities[1].authority(), Some("ROLE_USER"));
    }

    #[test]
    fn test_mapper_with_lower_case_conversion() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_convert_attribute_to_lower_case(true);

        let authorities =
            mapper.get_granted_authorities(&["ADMIN".to_string(), "USER".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("ROLE_admin"));
        assert_eq!(authorities[1].authority(), Some("ROLE_user"));
    }

    #[test]
    #[should_panic(expected = "Mapper is not properly configured")]
    fn test_mapper_with_both_case_conversions_panics() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_convert_attribute_to_upper_case(true);
        mapper.set_convert_attribute_to_lower_case(true);

        // This should panic because both flags are true
        let _authorities = mapper.get_granted_authorities(&["ADMIN".to_string()]);
    }

    #[test]
    fn test_mapper_with_add_prefix_if_already_existing() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_add_prefix_if_already_existing(true);

        let authorities =
            mapper.get_granted_authorities(&["ROLE_ADMIN".to_string(), "USER".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("ROLE_ROLE_ADMIN"));
        assert_eq!(authorities[1].authority(), Some("ROLE_USER"));
    }

    #[test]
    fn test_mapper_with_add_prefix_if_already_existing_false() {
        let mut mapper = SimpleAttributes2GrantedAuthoritiesMapper::default();
        mapper.set_add_prefix_if_already_existing(false);

        let authorities =
            mapper.get_granted_authorities(&["ROLE_ADMIN".to_string(), "USER".to_string()]);

        assert_eq!(authorities.len(), 2);
        assert_eq!(authorities[0].authority(), Some("ROLE_ADMIN"));
        assert_eq!(authorities[1].authority(), Some("ROLE_USER"));
    }
}
