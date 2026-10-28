//! Shiro's SHA credential matchers: the hash a stored password was taken with,
//! fixed to one algorithm.

use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        credential::{
            credentials_matcher::CredentialsMatcher,
            hashed_credentials_matcher::HashedCredentialsMatcher,
        },
    },
    crypto::hash::simple_hash::HashAlgorithm,
};

/// A matcher whose stored hashes are SHA-256.
///
/// Apache Shiro keeps one class per algorithm so a configuration names the hash it
/// stores; the port keeps the same names, and each of them is the
/// [`HashedCredentialsMatcher`] of its algorithm.
#[derive(Clone, Debug)]
pub struct Sha256CredentialsMatcher(HashedCredentialsMatcher);

/// A matcher whose stored hashes are SHA-384.
#[derive(Clone, Debug)]
pub struct Sha384CredentialsMatcher(HashedCredentialsMatcher);

/// A matcher whose stored hashes are SHA-512.
#[derive(Clone, Debug)]
pub struct Sha512CredentialsMatcher(HashedCredentialsMatcher);

macro_rules! sha_matcher {
    ($name:ident, $algorithm:expr, $default_iterations:expr) => {
        impl $name {
            /// Creates a matcher whose stored hashes are taken once.
            pub fn new() -> Self {
                Self::with_hash_iterations(1)
            }

            /// Creates a matcher whose stored hashes are taken a number of times.
            ///
            /// # Arguments
            ///
            /// * `hash_iterations` - How many times the digest was taken.
            pub fn with_hash_iterations(hash_iterations: u32) -> Self {
                Self(HashedCredentialsMatcher::new($algorithm).with_hash_iterations(hash_iterations))
            }

            /// Returns the matcher as the general hashed matcher.
            pub fn into_hashed_credentials_matcher(self) -> HashedCredentialsMatcher {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl From<$name> for HashedCredentialsMatcher {
            fn from(matcher: $name) -> Self {
                matcher.0
            }
        }

        impl CredentialsMatcher for $name {
            fn do_credentials_match(
                &self,
                token: &dyn AuthenticationToken,
                info: &dyn AuthenticationInfo,
            ) -> bool {
                self.0.do_credentials_match(token, info)
            }
        }
    };
}

sha_matcher!(Sha256CredentialsMatcher, HashAlgorithm::Sha256, 1);
sha_matcher!(Sha384CredentialsMatcher, HashAlgorithm::Sha384, 1);
sha_matcher!(Sha512CredentialsMatcher, HashAlgorithm::Sha512, 1);

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Sha256CredentialsMatcher, Sha384CredentialsMatcher, Sha512CredentialsMatcher};
    use crate::core::{
        authc::{
            credential::{
                credentials_matcher::CredentialsMatcher,
                hashed_credentials_matcher::HashedCredentialsMatcher,
            },
            simple_authentication_info::SimpleAuthenticationInfo,
            username_password_token::UsernamePasswordToken,
        },
        crypto::hash::simple_hash::{HashAlgorithm, SimpleHash},
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
    fn each_matcher_reads_the_hash_of_its_own_algorithm() {
        let sha256 = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1).to_hex();
        let sha384 = SimpleHash::new(HashAlgorithm::Sha384, b"secret", None, 1).to_hex();
        let sha512 = SimpleHash::new(HashAlgorithm::Sha512, b"secret", None, 1).to_hex();

        assert!(Sha256CredentialsMatcher::new().do_credentials_match(&token("secret"), &info(&sha256)));
        assert!(Sha384CredentialsMatcher::new().do_credentials_match(&token("secret"), &info(&sha384)));
        assert!(Sha512CredentialsMatcher::new().do_credentials_match(&token("secret"), &info(&sha512)));
    }

    #[test]
    fn a_hash_of_another_algorithm_does_not_match() {
        let sha512 = SimpleHash::new(HashAlgorithm::Sha512, b"secret", None, 1).to_hex();

        assert!(!Sha256CredentialsMatcher::new().do_credentials_match(&token("secret"), &info(&sha512)));
        assert!(!Sha512CredentialsMatcher::new().do_credentials_match(&token("wrong"), &info(&sha512)));
    }

    #[test]
    fn the_iterations_of_the_matcher_are_the_ones_of_the_stored_hash() {
        let stored = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 2).to_hex();

        assert!(Sha256CredentialsMatcher::with_hash_iterations(2)
            .do_credentials_match(&token("secret"), &info(&stored)));
        assert!(!Sha256CredentialsMatcher::with_hash_iterations(3)
            .do_credentials_match(&token("secret"), &info(&stored)));
    }

    #[test]
    fn a_matcher_hands_over_the_general_matcher_it_is() {
        let matcher: HashedCredentialsMatcher = Sha384CredentialsMatcher::new().into();

        assert_eq!(matcher.algorithm(), HashAlgorithm::Sha384);
        assert_eq!(
            Sha512CredentialsMatcher::default()
                .into_hashed_credentials_matcher()
                .algorithm(),
            HashAlgorithm::Sha512
        );
    }
}
