use std::sync::Arc;

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        credential::{credentials_matcher::CredentialsMatcher, credentials_text},
    },
    crypto::hash::password_service::DefaultPasswordService,
};

/// A matcher that verifies a password against the stored form of the password service.
///
/// The stored value is the modular crypt format of Shiro — the algorithm, the
/// iteration count and the salt travel with the hash — so the matcher reads the
/// parameters of the account out of the stored value instead of holding them
/// itself. That is what makes the stored password independent of the
/// configuration of the application that verifies it.
#[derive(Clone)]
pub struct PasswordMatcher {
    password_service: Arc<DefaultPasswordService>,
}

impl PasswordMatcher {
    /// Creates a matcher with the defaults of Shiro.
    pub fn new() -> Self {
        Self::with_password_service(Arc::new(DefaultPasswordService::new()))
    }

    /// Creates a matcher that verifies with the given service.
    ///
    /// # Arguments
    ///
    /// * `password_service` - The service that verifies the password.
    pub fn with_password_service(password_service: Arc<DefaultPasswordService>) -> Self {
        Self { password_service }
    }

    /// Returns the service of the matcher.
    pub fn password_service(&self) -> &Arc<DefaultPasswordService> {
        &self.password_service
    }
}

impl Default for PasswordMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialsMatcher for PasswordMatcher {
    fn do_credentials_match(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) -> bool {
        let (Some(submitted), Some(stored)) = (token.get_credentials(), info.get_credentials())
        else {
            return false;
        };

        self.password_service
            .password_matches(&credentials_text(&submitted), &credentials_text(&stored))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::PasswordMatcher;
    use crate::core::{
        authc::{
            credential::credentials_matcher::CredentialsMatcher,
            simple_authentication_info::SimpleAuthenticationInfo,
            username_password_token::UsernamePasswordToken,
        },
        crypto::hash::password_service::DefaultPasswordService,
        subject::simple_principal_collection::SimplePrincipalCollection,
        util::object::Object,
    };

    fn info(stored: &str) -> SimpleAuthenticationInfo {
        SimpleAuthenticationInfo::new(
            Arc::new(SimplePrincipalCollection::new(
                [Object::Str("root".to_string())],
                "iniRealm",
            )),
            Some(Object::Str(stored.to_string())),
        )
    }

    fn token(password: &str) -> UsernamePasswordToken {
        UsernamePasswordToken::new("root".to_string(), password.to_string(), false, None)
    }

    #[test]
    fn the_password_of_the_store_is_the_one_that_matches() {
        let service = DefaultPasswordService::new().with_iterations(1_000);
        let stored = service.encrypt_password_with_salt("secret", b"a-salt");
        let matcher = PasswordMatcher::with_password_service(Arc::new(service));

        assert!(matcher.do_credentials_match(&token("secret"), &info(&stored)));
        assert!(!matcher.do_credentials_match(&token("wrong"), &info(&stored)));
    }

    #[test]
    fn a_store_the_service_does_not_know_never_matches() {
        let matcher = PasswordMatcher::new();

        assert!(!matcher.do_credentials_match(&token("secret"), &info("secret")));
        assert!(!matcher.do_credentials_match(&token("secret"), &info("$shiro1$CRC32$1$a$b")));
    }

    #[test]
    fn an_account_without_a_password_never_matches() {
        let matcher = PasswordMatcher::new();
        let info = SimpleAuthenticationInfo::new(
            Arc::new(SimplePrincipalCollection::new(
                [Object::Str("root".to_string())],
                "iniRealm",
            )),
            None,
        );

        assert!(!matcher.do_credentials_match(&token("secret"), &info));
    }
}
