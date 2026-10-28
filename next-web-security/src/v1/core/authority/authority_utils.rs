use std::{collections::HashSet, sync::Arc};

use crate::core::{authority::SimpleGrantedAuthority, GrantedAuthority};

/// Utility method for manipulating GrantedAuthority collections etc.
/// Mainly intended for internal use
pub struct AuthorityUtils;

impl AuthorityUtils {
    pub fn no_authorities() -> Vec<Arc<dyn GrantedAuthority>> {
        Vec::new()
    }

    /// Creates a vector of [`GrantedAuthority`] objects from a comma-separated string
    /// representation (e.g. "ROLE_A, ROLE_B, ROLE_C").
    ///
    /// # Arguments
    /// * `authority_string` - The comma-separated string of authorities.
    ///
    /// # Returns
    /// A vector of [`SimpleGrantedAuthority`] created by tokenizing the string.
    pub fn comma_separated_string_to_authority_list(
        authority_string: &str,
    ) -> Vec<Arc<dyn GrantedAuthority>> {
        let tokens: Vec<&str> = authority_string
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        Self::create_authority_list(tokens)
    }

    /// Converts a collection of [`GrantedAuthority`] objects to a [`HashSet`] of authority strings.
    ///
    /// # Arguments
    /// * `user_authorities` - The authorities to convert.
    ///
    /// # Returns
    /// A [`HashSet`] containing the authority strings from each authority.
    pub fn authority_list_to_set<'a, I>(user_authorities: I) -> HashSet<String>
    where
        I: IntoIterator<Item = &'a dyn GrantedAuthority>,
    {
        user_authorities
            .into_iter()
            .filter_map(|auth| auth.authority().map(ToString::to_string))
            .collect()
    }

    /// Converts a collection of authority strings into a vector of [`GrantedAuthority`] objects.
    ///
    /// # Arguments
    /// * `authorities` - The authority strings to convert.
    ///
    /// # Returns
    /// A vector of [`SimpleGrantedAuthority`] objects.
    pub fn create_authority_list<S>(
        authorities: impl IntoIterator<Item = S>,
    ) -> Vec<Arc<dyn GrantedAuthority>>
    where
        S: AsRef<str>,
    {
        authorities
            .into_iter()
            .map(|authority| {
                Arc::new(SimpleGrantedAuthority::new(authority.as_ref()))
                    as Arc<dyn GrantedAuthority>
            })
            .collect()
    }
}
