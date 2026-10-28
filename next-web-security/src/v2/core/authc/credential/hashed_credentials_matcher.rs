use crate::core::{
    authc::{
        authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
        credential::{
            credentials_equal, credentials_matcher::CredentialsMatcher, credentials_text,
        },
    },
    crypto::{crypto_error::CryptoError, hash::simple_hash::SimpleHash},
};

/// A matcher that compares the hash of the submitted credentials with a stored
/// hash.
///
/// The stored value is the hash of the credentials, taken with the salt and the
/// number of iterations the account was created with; the matcher hashes what
/// the user submitted the same way and compares the two. A stored value is
/// hexadecimal by default, and the salt comes from the authentication info of
/// the account, never from the token.
///
/// The iterations are the cost of a verification: raising them makes a stolen
/// hash harder to attack and every login slower.
#[derive(Clone, Debug)]
pub struct HashedCredentialsMatcher {
    algorithm: crate::core::crypto::hash::simple_hash::HashAlgorithm,
    iterations: u32,
    stored_credentials_hex_encoded: bool,
    hash_salted: bool,
}

impl HashedCredentialsMatcher {
    /// Creates a matcher for the given algorithm.
    ///
    /// # Arguments
    ///
    /// * `algorithm` - The digest the stored hash was taken with.
    pub fn new(algorithm: crate::core::crypto::hash::simple_hash::HashAlgorithm) -> Self {
        Self {
            algorithm,
            iterations: SimpleHash::DEFAULT_ITERATIONS,
            stored_credentials_hex_encoded: true,
            hash_salted: false,
        }
    }

    /// Creates a matcher whose stored hashes are SHA-256, the algorithm a new
    /// account should use.
    ///
    /// # Arguments
    ///
    /// * `iterations` - How many times the digest is taken.
    pub fn sha256(iterations: u32) -> Self {
        Self::new(
            crate::core::crypto::hash::simple_hash::HashAlgorithm::Sha256,
        )
        .with_hash_iterations(iterations)
    }

    /// Creates a matcher whose stored hashes are SHA-384.
    ///
    /// # Arguments
    ///
    /// * `iterations` - How many times the digest is taken.
    pub fn sha384(iterations: u32) -> Self {
        Self::new(
            crate::core::crypto::hash::simple_hash::HashAlgorithm::Sha384,
        )
        .with_hash_iterations(iterations)
    }

    /// Creates a matcher whose stored hashes are SHA-512.
    ///
    /// # Arguments
    ///
    /// * `iterations` - How many times the digest is taken.
    pub fn sha512(iterations: u32) -> Self {
        Self::new(
            crate::core::crypto::hash::simple_hash::HashAlgorithm::Sha512,
        )
        .with_hash_iterations(iterations)
    }

    /// Sets the algorithm by the name Shiro uses for it.
    ///
    /// # Arguments
    ///
    /// * `algorithm_name` - `MD5`, `SHA-1`, `SHA-256`, `SHA-384`, or `SHA-512`.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::UnknownAlgorithm`] for a name the port does not
    /// compute.
    pub fn with_hash_algorithm_name(
        mut self,
        algorithm_name: &str,
    ) -> Result<Self, CryptoError> {
        let algorithm =
            crate::core::crypto::hash::simple_hash::HashAlgorithm::from_name(algorithm_name)
                .ok_or_else(|| CryptoError::UnknownAlgorithm(algorithm_name.to_string()))?;
        self.algorithm = algorithm;

        Ok(self)
    }

    /// Sets how many times the digest is taken.
    ///
    /// # Arguments
    ///
    /// * `iterations` - The count; `0` is read as `1`.
    pub fn with_hash_iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations.max(SimpleHash::DEFAULT_ITERATIONS);
        self
    }

    /// Sets whether the stored hash is hexadecimal instead of base64.
    ///
    /// # Arguments
    ///
    /// * `hex_encoded` - Whether the store holds the hash in hexadecimal.
    pub fn with_stored_credentials_hex_encoded(mut self, hex_encoded: bool) -> Self {
        self.stored_credentials_hex_encoded = hex_encoded;
        self
    }

    /// Sets whether the token principal is used as the salt.
    ///
    /// Apache Shiro keeps this for a store that predates the salted
    /// authentication info; a new store takes the salt from the account, which
    /// is the default here.
    ///
    /// # Arguments
    ///
    /// * `hash_salted` - Whether the principal of the token is the salt.
    pub fn with_hash_salted(mut self, hash_salted: bool) -> Self {
        self.hash_salted = hash_salted;
        self
    }

    /// Returns the algorithm the stored hash was taken with.
    pub fn algorithm(&self) -> crate::core::crypto::hash::simple_hash::HashAlgorithm {
        self.algorithm
    }

    /// Returns how many times the digest is taken.
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Returns the hash of the credentials of the token.
    ///
    /// # Arguments
    ///
    /// * `token` - The token of the login.
    /// * `info` - The authentication info of the account.
    fn hash_submitted(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) -> Option<SimpleHash> {
        let submitted = token.get_credentials()?;
        let submitted = credentials_text(&submitted);

        let salt = match info.get_credentials_salt() {
            Some(salt) => Some(salt.to_vec()),
            None if self.hash_salted => {
                Some(credentials_text(&token.get_principal()).into_bytes())
            }
            None => None,
        };

        Some(SimpleHash::new(
            self.algorithm,
            submitted.as_bytes(),
            salt.as_deref(),
            self.iterations,
        ))
    }

    /// Returns the hash as the store holds it.
    ///
    /// # Arguments
    ///
    /// * `hash` - The hash of the submitted credentials.
    fn stored_form(&self, hash: &SimpleHash) -> String {
        if self.stored_credentials_hex_encoded {
            hash.to_hex()
        } else {
            hash.to_base64()
        }
    }
}

impl Default for HashedCredentialsMatcher {
    fn default() -> Self {
        // SHA-256 is the algorithm a new store should use; MD5 and SHA-1 are part
        // of the port only to verify a value that was stored with them.
        Self::sha256(SimpleHash::DEFAULT_ITERATIONS)
    }
}

impl CredentialsMatcher for HashedCredentialsMatcher {
    fn do_credentials_match(
        &self,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) -> bool {
        let Some(stored) = info.get_credentials() else {
            return false;
        };
        let Some(hash) = self.hash_submitted(token, info) else {
            return false;
        };

        let computed = self.stored_form(&hash);
        let stored = credentials_text(&stored);

        // A hexadecimal hash is written in either case; base64 is not.
        if self.stored_credentials_hex_encoded {
            credentials_equal(
                &computed.to_ascii_lowercase(),
                &stored.to_ascii_lowercase(),
            )
        } else {
            credentials_equal(&computed, &stored)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::HashedCredentialsMatcher;
    use crate::core::{
        authc::{
            credential::credentials_matcher::CredentialsMatcher,
            simple_authentication_info::SimpleAuthenticationInfo,
            username_password_token::UsernamePasswordToken,
        },
        crypto::hash::simple_hash::{HashAlgorithm, SimpleHash},
        subject::simple_principal_collection::SimplePrincipalCollection,
        util::object::Object,
    };

    fn info(credentials: &str, salt: Option<&[u8]>) -> SimpleAuthenticationInfo {
        let principals = Arc::new(SimplePrincipalCollection::new(
            [Object::Str("root".to_string())],
            "iniRealm",
        ));
        let info = SimpleAuthenticationInfo::new(
            principals,
            Some(Object::Str(credentials.to_string())),
        );

        match salt {
            Some(salt) => info.with_salt(salt.to_vec()),
            None => info,
        }
    }

    fn token(password: &str) -> UsernamePasswordToken {
        UsernamePasswordToken::new("root".to_string(), password.to_string(), false, None)
    }

    #[test]
    fn the_hash_of_the_submitted_password_matches_the_stored_hash() {
        let stored = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1).to_hex();
        let matcher = HashedCredentialsMatcher::sha256(1);

        assert!(matcher.do_credentials_match(&token("secret"), &info(&stored, None)));
        assert!(!matcher.do_credentials_match(&token("wrong"), &info(&stored, None)));
    }

    #[test]
    fn the_salt_of_the_account_is_mixed_into_the_hash() {
        let salt = b"a-per-account-salt";
        let stored = SimpleHash::new(HashAlgorithm::Sha256, b"secret", Some(salt), 2).to_hex();
        let matcher = HashedCredentialsMatcher::sha256(2);

        assert!(matcher.do_credentials_match(&token("secret"), &info(&stored, Some(salt))));
        // A hash taken without the salt of the account does not match.
        assert!(!matcher.do_credentials_match(&token("secret"), &info(&stored, None)));
        // Nor does a different salt.
        assert!(!matcher.do_credentials_match(&token("secret"), &info(&stored, Some(b"other"))));
    }

    #[test]
    fn iterations_are_part_of_the_stored_hash() {
        let stored = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 2).to_hex();

        assert!(HashedCredentialsMatcher::sha256(2)
            .do_credentials_match(&token("secret"), &info(&stored, None)));
        assert!(!HashedCredentialsMatcher::sha256(1)
            .do_credentials_match(&token("secret"), &info(&stored, None)));
    }

    #[test]
    fn a_base64_stored_hash_is_matched_in_base64() {
        let stored = SimpleHash::new(HashAlgorithm::Sha512, b"secret", None, 1).to_base64();
        let matcher = HashedCredentialsMatcher::sha512(1).with_stored_credentials_hex_encoded(false);

        assert!(matcher.do_credentials_match(&token("secret"), &info(&stored, None)));
    }

    #[test]
    fn the_stored_hash_may_be_written_in_upper_case() {
        let stored = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1)
            .to_hex()
            .to_ascii_uppercase();

        assert!(HashedCredentialsMatcher::sha256(1)
            .do_credentials_match(&token("secret"), &info(&stored, None)));
    }

    #[test]
    fn an_account_without_stored_credentials_is_never_matched() {
        let principals = Arc::new(SimplePrincipalCollection::new(
            [Object::Str("root".to_string())],
            "iniRealm",
        ));

        assert!(!HashedCredentialsMatcher::sha256(1)
            .do_credentials_match(&token("secret"), &SimpleAuthenticationInfo::new(principals, None)));
    }

    #[test]
    fn the_algorithm_is_set_by_the_name_shiro_uses() {
        let matcher = HashedCredentialsMatcher::default()
            .with_hash_algorithm_name("SHA-512")
            .expect("the algorithm is known");

        assert_eq!(matcher.algorithm(), HashAlgorithm::Sha512);
        assert!(HashedCredentialsMatcher::default()
            .with_hash_algorithm_name("crc32")
            .is_err());
    }
}
