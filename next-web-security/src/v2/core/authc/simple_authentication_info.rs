//! Shiro's in-memory authentication info.

use std::{fmt::Display, sync::Arc};

use crate::core::{
    authc::authentication_info::AuthenticationInfo,
    subject::principal_collection::PrincipalCollection,
    util::object::Object,
};

/// The identity and the credentials a realm looked up for a token.
///
/// The principals identify the subject; the credentials are what the token is
/// compared against by the credentials matcher of the realm.
#[derive(Clone)]
pub struct SimpleAuthenticationInfo {
    principals: Arc<dyn PrincipalCollection>,
    credentials: Option<Object>,
    credentials_salt: Option<Vec<u8>>,
}

impl SimpleAuthenticationInfo {
    /// Creates the authentication info of a login.
    ///
    /// # Arguments
    ///
    /// * `principals` - The principals the realm authenticated.
    /// * `credentials` - The credentials the token is matched against.
    pub fn new(principals: Arc<dyn PrincipalCollection>, credentials: Option<Object>) -> Self {
        Self {
            principals,
            credentials,
            credentials_salt: None,
        }
    }

    /// Sets the salt the stored credentials were hashed with.
    ///
    /// # Arguments
    ///
    /// * `salt` - The salt of the account.
    pub fn with_salt(mut self, salt: Vec<u8>) -> Self {
        self.credentials_salt = Some(salt);
        self
    }

    /// Returns the salt the stored credentials were hashed with.
    pub fn credentials_salt(&self) -> Option<&[u8]> {
        self.credentials_salt.as_deref()
    }

    /// Returns the principals as their concrete collection.
    pub fn principals(&self) -> &Arc<dyn PrincipalCollection> {
        &self.principals
    }

    /// Returns the credentials the token is matched against.
    pub fn credentials(&self) -> Option<&Object> {
        self.credentials.as_ref()
    }
}

impl AuthenticationInfo for SimpleAuthenticationInfo {
    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        Some(&self.principals)
    }

    fn get_credentials(&self) -> Option<&Object> {
        self.credentials.as_ref()
    }

    fn get_credentials_salt(&self) -> Option<&[u8]> {
        self.credentials_salt.as_deref()
    }
}

impl Display for SimpleAuthenticationInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "principals={}, credentials={}",
            self.principals,
            match &self.credentials {
                Some(credentials) => credentials.to_string(),
                None => "null".to_string(),
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SimpleAuthenticationInfo;
    use crate::core::{
        authc::authentication_info::AuthenticationInfo,
        subject::{
            principal_collection::PrincipalCollection,
            simple_principal_collection::SimplePrincipalCollection,
        },
        util::object::Object,
    };

    fn info(credentials: Option<Object>) -> SimpleAuthenticationInfo {
        let principals: Arc<dyn PrincipalCollection> =
            Arc::new(SimplePrincipalCollection::new(
                [Object::Str("root".to_string())],
                "iniRealm",
            ));

        SimpleAuthenticationInfo::new(principals, credentials)
    }

    #[test]
    fn the_principals_and_the_credentials_are_readable() {
        let info = info(Some(Object::Str("secret".to_string())));

        assert_eq!(
            info.get_principals()
                .and_then(|principals| principals.get_primary_principal())
                .map(ToString::to_string),
            Some("root".to_string())
        );
        assert_eq!(
            info.get_credentials().map(ToString::to_string),
            Some("secret".to_string())
        );
        assert!(info.to_string().contains("principals=[root]"));
    }

    #[test]
    fn credentials_are_optional() {
        let info = info(None);

        assert!(info.get_credentials().is_none());
        assert!(info.to_string().contains("credentials=null"));
    }
}
