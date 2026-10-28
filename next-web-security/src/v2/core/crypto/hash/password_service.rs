//! Shiro's stored password: a hashed password that carries its own parameters.
//!
//! A password is stored in the modular crypt format of Shiro:
//!
//! ```text
//! $shiro1$ALGORITHM$ITERATIONS$base64Salt$base64Hash
//! ```
//!
//! The algorithm, the iteration count and the salt are part of the stored text,
//! so a verification reproduces the hash from the stored value alone. That is
//! what lets the parameters of an application change without locking every
//! account out: a password keeps verifying with the parameters it was stored
//! with.

use base64::{Engine, engine::general_purpose::STANDARD};
use rand::RngCore;

use crate::core::crypto::{
    crypto_error::CryptoError,
    hash::simple_hash::{HashAlgorithm, SimpleHash},
};

/// The prefix of a stored password.
const SHIRO1_PREFIX: &str = "$shiro1$";

/// A password service: hashes a password for storage, and verifies one against
/// a stored value.
///
/// Apache Shiro's `DefaultPasswordService` in front of its `DefaultHashService`:
/// SHA-512, 500 000 iterations and a random salt for a new password, and the
/// parameters of the stored value for a verification.
#[derive(Clone, Debug)]
pub struct DefaultPasswordService {
    algorithm: HashAlgorithm,
    iterations: u32,
    salt_length: usize,
}

impl DefaultPasswordService {
    /// The number of iterations a new password is hashed with.
    pub const DEFAULT_ITERATIONS: u32 = 500_000;

    /// The length of the random salt of a new password, in bytes.
    pub const DEFAULT_SALT_LENGTH: usize = 16;

    /// Creates a service with the defaults of Shiro.
    pub fn new() -> Self {
        Self {
            algorithm: HashAlgorithm::Sha512,
            iterations: Self::DEFAULT_ITERATIONS,
            salt_length: Self::DEFAULT_SALT_LENGTH,
        }
    }

    /// Sets the algorithm a new password is hashed with.
    ///
    /// # Arguments
    ///
    /// * `algorithm` - The digest to use.
    pub fn with_algorithm(mut self, algorithm: HashAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Sets how many times a new password is hashed.
    ///
    /// # Arguments
    ///
    /// * `iterations` - The count; `0` is read as `1`.
    pub fn with_iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations.max(SimpleHash::DEFAULT_ITERATIONS);
        self
    }

    /// Sets the length of the random salt of a new password.
    ///
    /// # Arguments
    ///
    /// * `salt_length` - The length in bytes.
    pub fn with_salt_length(mut self, salt_length: usize) -> Self {
        self.salt_length = salt_length;
        self
    }

    /// Returns the algorithm a new password is hashed with.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Returns how many times a new password is hashed.
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Returns the stored form of a password.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The password the user chose.
    pub fn encrypt_password(&self, plaintext: &str) -> String {
        let mut salt = vec![0u8; self.salt_length];
        rand::thread_rng().fill_bytes(&mut salt);

        self.encrypt_password_with_salt(plaintext, &salt)
    }

    /// Returns the stored form of a password, with the given salt.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The password the user chose.
    /// * `salt` - The salt of the account.
    pub fn encrypt_password_with_salt(&self, plaintext: &str, salt: &[u8]) -> String {
        let hash = SimpleHash::new(self.algorithm, plaintext.as_bytes(), Some(salt), self.iterations);

        format!(
            "{SHIRO1_PREFIX}{}${}${}${}",
            self.algorithm.name(),
            self.iterations,
            STANDARD.encode(salt),
            STANDARD.encode(hash.bytes())
        )
    }

    /// Returns whether the submitted password is the stored one.
    ///
    /// # Arguments
    ///
    /// * `submitted` - The password the user submitted.
    /// * `stored` - The stored form of the password.
    ///
    /// # Errors
    ///
    /// Returns an error when the stored value does not carry its parameters.
    pub fn password_matches(
        &self,
        submitted: &str,
        stored: &str,
    ) -> Result<bool, CryptoError> {
        let (algorithm, iterations, salt, hash) = Self::parse(stored)?;

        let computed = SimpleHash::new(algorithm, submitted.as_bytes(), Some(&salt), iterations);

        Ok(crate::core::authc::credential::credentials_equal(
            &STANDARD.encode(computed.bytes()),
            &STANDARD.encode(hash),
        ))
    }

    /// Reads the parameters a stored password carries.
    ///
    /// # Arguments
    ///
    /// * `stored` - The stored form of the password.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::UnknownAlgorithm`] for a value whose algorithm the
    /// port does not compute, and [`CryptoError::DecryptionError`] for a value
    /// that is not the format of this service.
    pub fn parse(
        stored: &str,
    ) -> Result<(HashAlgorithm, u32, Vec<u8>, Vec<u8>), CryptoError> {
        let stored = stored.trim();
        let Some(rest) = stored.strip_prefix(SHIRO1_PREFIX) else {
            return Err(CryptoError::DecryptionError(format!(
                "The stored password does not start with `{SHIRO1_PREFIX}`."
            )));
        };

        let parts: Vec<&str> = rest.split('$').collect();
        if parts.len() != 4 {
            return Err(CryptoError::DecryptionError(
                "The stored password does not carry an algorithm, an iteration count, a salt and \
                 a hash."
                    .to_string(),
            ));
        }

        let algorithm = HashAlgorithm::from_name(parts[0])
            .ok_or_else(|| CryptoError::UnknownAlgorithm(parts[0].to_string()))?;
        let iterations: u32 = parts[1].parse().map_err(|_| {
            CryptoError::DecryptionError(format!(
                "The stored password names [{}] iterations.",
                parts[1]
            ))
        })?;
        let salt = STANDARD.decode(parts[2]).map_err(|_| {
            CryptoError::DecryptionError("The salt of the stored password is not base64.".to_string())
        })?;
        let hash = STANDARD.decode(parts[3]).map_err(|_| {
            CryptoError::DecryptionError("The hash of the stored password is not base64.".to_string())
        })?;

        Ok((algorithm, iterations.max(SimpleHash::DEFAULT_ITERATIONS), salt, hash))
    }
}

impl Default for DefaultPasswordService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::DefaultPasswordService;
    use crate::core::crypto::crypto_error::CryptoError;

    fn service() -> DefaultPasswordService {
        // A low iteration count keeps the tests quick; a deployment keeps the
        // default of Shiro.
        DefaultPasswordService::new().with_iterations(1_000)
    }

    #[test]
    fn a_stored_password_carries_its_parameters() {
        let stored = service().encrypt_password_with_salt("secret", b"a-salt");

        assert!(stored.starts_with("$shiro1$SHA-512$1000$"));
        let (algorithm, iterations, salt, hash) =
            DefaultPasswordService::parse(&stored).expect("the value is the format");
        assert_eq!(algorithm.name(), "SHA-512");
        assert_eq!(iterations, 1_000);
        assert_eq!(salt, b"a-salt");
        assert!(!hash.is_empty());
    }

    #[test]
    fn the_password_it_stored_is_the_one_it_verifies() {
        let service = service();
        let stored = service.encrypt_password("secret");

        assert!(service
            .password_matches("secret", &stored)
            .expect("the stored value is readable"));
        assert!(!service
            .password_matches("wrong", &stored)
            .expect("the stored value is readable"));
    }

    #[test]
    fn a_new_password_gets_a_random_salt() {
        let service = service();
        let first = service.encrypt_password("secret");
        let second = service.encrypt_password("secret");

        assert_ne!(first, second, "the salt of two stores differs");
        assert!(service.password_matches("secret", &first).unwrap_or_default());
        assert!(service.password_matches("secret", &second).unwrap_or_default());
    }

    #[test]
    fn a_password_keeps_verifying_with_the_parameters_it_was_stored_with() {
        let stored = service().encrypt_password_with_salt("secret", b"a-salt");

        // The configuration of the application changed since the store.
        let changed = DefaultPasswordService::new()
            .with_iterations(2_000)
            .with_algorithm(crate::core::crypto::hash::simple_hash::HashAlgorithm::Sha256);

        assert!(changed
            .password_matches("secret", &stored)
            .expect("the stored value carries its own parameters"));
    }

    #[test]
    fn a_value_that_is_not_the_format_is_refused() {
        let service = service();

        assert!(matches!(
            service.password_matches("secret", "secret"),
            Err(CryptoError::DecryptionError(_))
        ));
        assert!(matches!(
            DefaultPasswordService::parse("$shiro1$CRC32$10$c2FsdA==$aGFzaA=="),
            Err(CryptoError::UnknownAlgorithm(_))
        ));
        assert!(matches!(
            DefaultPasswordService::parse("$shiro1$SHA-512$ten$c2FsdA==$aGFzaA=="),
            Err(CryptoError::DecryptionError(_))
        ));
    }

    #[test]
    fn a_tampered_hash_does_not_match() {
        let service = service();
        let stored = service.encrypt_password_with_salt("secret", b"a-salt");
        let tampered = format!("{}A", stored);

        assert!(!service.password_matches("secret", &tampered).unwrap_or_default());
    }
}
