use std::collections::HashMap;

use crate::authorization::RequiredAuthoritiesRepository;

/// A Map-based implementation of [`RequiredAuthoritiesRepository`].
#[derive(Debug, Default)]
pub struct MapRequiredAuthoritiesRepository {
    username_to_authorities: HashMap<String, Vec<String>>,
}

impl MapRequiredAuthoritiesRepository {
    /// Saves the required authorities for a given username.
    ///
    /// If the username already exists, the authorities will be overwritten.
    pub fn save_required_authorities(
        &mut self,
        username: impl Into<String>,
        authorities: Vec<String>,
    ) {
        let username = username.into();
        assert!(!username.is_empty(), "username cannot be empty");
        assert!(!authorities.is_empty(), "authorities cannot be empty");
        self.username_to_authorities.insert(username, authorities);
    }

    /// Deletes the required authorities for a given username.
    ///
    /// If the username does not exist, this operation does nothing.
    pub fn delete_required_authorities(&mut self, username: &str) {
        assert!(!username.is_empty(), "username cannot be empty");
        self.username_to_authorities.remove(username);
    }
}

impl RequiredAuthoritiesRepository for MapRequiredAuthoritiesRepository {
    fn find_required_authorities(&self, username: &str) -> Vec<&str> {
        assert!(!username.is_empty(), "username cannot be empty");
        self.username_to_authorities
            .get(username)
            .map(|s| s.iter().map(AsRef::as_ref).collect())
            .unwrap_or_default()
    }
}
