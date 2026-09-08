use std::sync::Arc;

use crate::core::GrantedAuthority;

/// Interface to be implemented by classes that can map a list of security attributes
/// (such as roles or group names) to a collection of Spring Security
/// `GrantedAuthority`s.
/// ```
pub trait Attributes2GrantedAuthoritiesMapper: Send + Sync {
    /// Implementations of this method should map the given collection of attributes to a
    /// collection of `GrantedAuthorities`.
    ///
    /// There are no restrictions for the mapping process:
    /// - A single attribute can be mapped to multiple `GrantedAuthorities`
    /// - All attributes can be mapped to a single `GrantedAuthority`
    /// - Some attributes may not be mapped
    /// - The mapping can be lossy or lossless
    ///
    /// # Arguments
    /// * `attributes` - The attributes to be mapped (e.g., roles, group names)
    ///
    /// # Returns
    /// A collection of authorities created from the attributes.
    fn get_granted_authorities(&self, attributes: &[String]) -> Vec<Arc<dyn GrantedAuthority>>;
}
