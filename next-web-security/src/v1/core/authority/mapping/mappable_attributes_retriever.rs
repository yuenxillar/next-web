use std::collections::HashSet;

/// Interface to be implemented by classes that can retrieve a list of mappable security
/// attribute strings (for example the list of all available J2EE roles in a web or EJB
/// application).
///
/// This trait is typically used in conjunction with [`Attributes2GrantedAuthoritiesMapper`]
/// to discover what attributes are available for mapping.
pub trait MappableAttributesRetriever {
    /// Implementations of this method should return a set of all string attributes which
    /// can be mapped to `GrantedAuthority`s.
    ///
    /// # Returns
    /// A set of all mappable attributes (e.g., roles, permissions, group names).
    fn get_mappable_attributes(&self) -> &HashSet<String>;
}
