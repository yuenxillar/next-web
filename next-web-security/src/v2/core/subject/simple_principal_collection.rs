//! Shiro's in-memory principal collection.

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use next_web_core::Id;

use crate::core::{
    subject::principal_collection::PrincipalCollection, util::object::Object,
};

/// The principals of a subject, kept in memory and grouped by realm name.
///
/// The first principal added becomes the primary principal of the subject, the
/// one an application usually shows and looks the account up by.
///
/// ```
/// use next_web_security::core::subject::{
///     principal_collection::PrincipalCollection,
///     simple_principal_collection::SimplePrincipalCollection,
/// };
/// use next_web_security::core::util::object::Object;
///
/// let mut principals = SimplePrincipalCollection::default();
/// principals.add(Object::Str("root".to_string()), "iniRealm");
///
/// assert_eq!(principals.get_primary_principal().map(|p| p.to_string()), Some("root".into()));
/// assert!(!principals.is_empty());
/// ```
#[derive(Clone, Default)]
pub struct SimplePrincipalCollection {
    /// The principals of each realm, in the order they were added.
    principals_by_realm: Vec<(String, Vec<Object>)>,
    /// The identifier of the collection, derived from its principals.
    identifier: String,
}

impl SimplePrincipalCollection {
    /// Creates a collection that holds the given principals of one realm.
    ///
    /// # Arguments
    ///
    /// * `principals` - The principals the realm authenticated.
    /// * `realm_name` - The name of the realm that authenticated them.
    pub fn new(
        principals: impl IntoIterator<Item = Object>,
        realm_name: impl ToString,
    ) -> Self {
        let mut collection = Self::default();
        for principal in principals {
            collection.add(principal, realm_name.to_string());
        }

        collection
    }

    /// Creates a collection with one principal.
    ///
    /// # Arguments
    ///
    /// * `principal` - The principal the realm authenticated.
    /// * `realm_name` - The name of the realm that authenticated it.
    pub fn from_primary(principal: Object, realm_name: impl ToString) -> Self {
        Self::new([principal], realm_name)
    }

    /// Adds a principal of the given realm.
    ///
    /// # Arguments
    ///
    /// * `principal` - The principal the realm authenticated.
    /// * `realm_name` - The name of the realm that authenticated it.
    pub fn add(&mut self, principal: Object, realm_name: impl ToString) {
        let realm_name = realm_name.to_string();

        match self
            .principals_by_realm
            .iter_mut()
            .find(|(name, _)| name == &realm_name)
        {
            Some((_, principals)) => principals.push(principal),
            None => self
                .principals_by_realm
                .push((realm_name, vec![principal])),
        }

        self.identifier = self
            .principals_by_realm
            .iter()
            .flat_map(|(_, principals)| principals.iter().map(ToString::to_string))
            .collect::<Vec<_>>()
            .join(",");
    }

    /// Returns every principal of the collection, in the order they were added.
    pub fn principals(&self) -> Vec<&Object> {
        self.principals_by_realm
            .iter()
            .flat_map(|(_, principals)| principals.iter())
            .collect()
    }

    /// Returns the principals the given realm contributed.
    ///
    /// # Arguments
    ///
    /// * `realm_name` - The name of the realm.
    pub fn from_realm(&self, realm_name: &str) -> Option<&[Object]> {
        self.principals_by_realm
            .iter()
            .find(|(name, _)| name == realm_name)
            .map(|(_, principals)| principals.as_slice())
    }

    /// Returns the principals keyed by the realm that contributed them.
    pub fn by_realm(&self) -> HashMap<&str, &[Object]> {
        self.principals_by_realm
            .iter()
            .map(|(name, principals)| (name.as_str(), principals.as_slice()))
            .collect()
    }
}

impl PrincipalCollection for SimplePrincipalCollection {
    fn get_primary_principal(&self) -> Option<&Object> {
        self.principals_by_realm
            .first()
            .and_then(|(_, principals)| principals.first())
    }

    fn get_realm_names(&self) -> Option<HashSet<&str>> {
        if self.principals_by_realm.is_empty() {
            return None;
        }

        Some(
            self.principals_by_realm
                .iter()
                .map(|(name, _)| name.as_str())
                .collect(),
        )
    }

    fn is_empty(&self) -> bool {
        self.principals_by_realm.is_empty()
    }
}

impl Id for SimplePrincipalCollection {
    fn id(&self) -> &str {
        &self.identifier
    }
}

impl Display for SimplePrincipalCollection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.identifier)
    }
}

#[cfg(test)]
mod tests {
    use super::SimplePrincipalCollection;
    use crate::core::{subject::principal_collection::PrincipalCollection, util::object::Object};
    use next_web_core::Id;

    fn name(value: &str) -> Object {
        Object::Str(value.to_string())
    }

    #[test]
    fn the_first_principal_is_the_primary_one() {
        let mut principals = SimplePrincipalCollection::default();
        principals.add(name("root"), "iniRealm");
        principals.add(name("administrator"), "iniRealm");

        assert_eq!(
            principals.get_primary_principal().map(ToString::to_string),
            Some("root".to_string())
        );
        assert_eq!(principals.principals().len(), 2);
    }

    #[test]
    fn the_principals_are_grouped_by_realm() {
        let principals = SimplePrincipalCollection::new([name("root")], "iniRealm");
        let mut principals = principals;
        principals.add(name("cn=root"), "ldapRealm");

        let realm_names = principals
            .get_realm_names()
            .expect("the collection names its realms");
        assert!(realm_names.contains("iniRealm"));
        assert!(realm_names.contains("ldapRealm"));

        assert_eq!(
            principals.from_realm("ldapRealm").map(|p| p.len()),
            Some(1)
        );
        assert!(principals.from_realm("unknown").is_none());
    }

    #[test]
    fn an_empty_collection_reports_no_primary_principal_and_no_realm() {
        let principals = SimplePrincipalCollection::default();

        assert!(principals.is_empty());
        assert!(principals.get_primary_principal().is_none());
        assert!(principals.get_realm_names().is_none());
        assert_eq!(principals.id(), "");
        assert_eq!(principals.to_string(), "[]");
    }

    #[test]
    fn the_identifier_lists_the_principals() {
        let principals = SimplePrincipalCollection::new([name("root"), name("admin")], "iniRealm");

        assert_eq!(principals.id(), "root,admin");
        assert_eq!(principals.to_string(), "[root,admin]");
    }
}
