use std::sync::Arc;

use crate::core::GrantedAuthority;

/// Mapping trait which can be injected into the authentication layer to convert the
/// authorities loaded from storage into those which will be used in the
/// [`Authentication`](crate::core::Authentication) object.
///
/// This trait allows for transformation of granted authorities, such as:
/// - Adding or removing prefixes (e.g., "ROLE_" to "SCOPE_")
/// - Case conversion (upper/lower case normalization)
/// - Filtering or mapping authorities based on business rules
/// - Combining or splitting authorities
pub trait GrantedAuthoritiesMapper: Send + Sync {
    /// Maps the given collection of authorities to a new collection of authorities.
    ///
    /// This method allows for transformation of authorities, such as:
    /// - Adding or removing prefixes
    /// - Case conversion
    /// - Filtering
    /// - Combining or splitting
    ///
    /// # Arguments
    /// * `authorities` - The authorities to be mapped (as a slice of trait objects)
    ///
    /// # Returns
    /// A vector of mapped authorities.
    fn map_authorities(
        &self,
        authorities: &[Arc<dyn GrantedAuthority>],
    ) -> Vec<Arc<dyn GrantedAuthority>>;
}
