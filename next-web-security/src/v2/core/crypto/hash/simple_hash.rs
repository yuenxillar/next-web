//! Shiro's salted, iterated hash of a value.
//!
//! The algorithm and the number of iterations are part of the stored value,
//! because the hash of a password is only reproducible with the same salt and
//! the same count.

use md5 as md5_crate;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::core::crypto::crypto_error::CryptoError;

/// The digest a hash is computed with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HashAlgorithm {
    /// MD5 — only for a stored value that already uses it.
    Md5,
    /// SHA-1 — only for a stored value that already uses it.
    Sha1,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
}

impl HashAlgorithm {
    /// Returns the algorithm of the given name, in the spelling Shiro uses.
    ///
    /// # Arguments
    ///
    /// * `name` - `MD5`, `SHA-1`, `SHA-256`, `SHA-384`, or `SHA-512`, in any case.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().as_str() {
            "MD5" => Some(Self::Md5),
            "SHA-1" | "SHA1" => Some(Self::Sha1),
            "SHA-256" | "SHA256" => Some(Self::Sha256),
            "SHA-384" | "SHA384" => Some(Self::Sha384),
            "SHA-512" | "SHA512" => Some(Self::Sha512),
            _ => None,
        }
    }

    /// Returns the name of the algorithm, in the spelling Shiro uses.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }

    /// Returns the digest of the bytes, without a salt.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes to digest.
    fn digest(&self, bytes: &[u8]) -> Vec<u8> {
        match self {
            Self::Md5 => md5_crate::compute(bytes).0.to_vec(),
            Self::Sha1 => digest_with::<Sha1>(bytes),
            Self::Sha256 => digest_with::<Sha256>(bytes),
            Self::Sha384 => digest_with::<Sha384>(bytes),
            Self::Sha512 => digest_with::<Sha512>(bytes),
        }
    }
}

/// Returns the digest of the bytes with a [`Digest`] implementation.
///
/// # Arguments
///
/// * `bytes` - The bytes to digest.
fn digest_with<D: Digest>(bytes: &[u8]) -> Vec<u8> {
    let mut digest = D::new();
    digest.update(bytes);

    digest.finalize().to_vec()
}

/// The salted, iterated hash of a value.
///
/// The algorithm of Apache Shiro's `SimpleHash`: the salt is fed to the digest
/// before the value, the digest is taken once, and the remaining iterations
/// digest the previous digest — so the work of a verification grows with the
/// count while the stored value stays one digest long.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimpleHash {
    algorithm: HashAlgorithm,
    iterations: u32,
    bytes: Vec<u8>,
}

impl SimpleHash {
    /// The number of iterations a hash is taken with when none is asked for.
    pub const DEFAULT_ITERATIONS: u32 = 1;

    /// Hashes the given bytes.
    ///
    /// # Arguments
    ///
    /// * `algorithm` - The digest to use.
    /// * `source` - The bytes to hash, usually the submitted credentials.
    /// * `salt` - The salt the hash is mixed with, when the account has one.
    /// * `iterations` - How many times the digest is taken; `0` is read as `1`.
    pub fn new(
        algorithm: HashAlgorithm,
        source: &[u8],
        salt: Option<&[u8]>,
        iterations: u32,
    ) -> Self {
        let iterations = iterations.max(Self::DEFAULT_ITERATIONS);
        let salt = salt.unwrap_or_default();

        // The first digest mixes the salt in, the ones after it digest their own
        // output; the salt is not repeated, which is what Shiro does.
        let mut bytes = if salt.is_empty() {
            algorithm.digest(source)
        } else {
            let mut salted = Vec::with_capacity(salt.len() + source.len());
            salted.extend_from_slice(salt);
            salted.extend_from_slice(source);

            algorithm.digest(&salted)
        };

        for _ in 1..iterations {
            bytes = algorithm.digest(&bytes);
        }

        Self {
            algorithm,
            iterations,
            bytes,
        }
    }

    /// Hashes the given bytes with an algorithm named the way Shiro names it.
    ///
    /// # Arguments
    ///
    /// * `algorithm_name` - `MD5`, `SHA-1`, `SHA-256`, `SHA-384`, or `SHA-512`.
    /// * `source` - The bytes to hash.
    /// * `salt` - The salt the hash is mixed with.
    /// * `iterations` - How many times the digest is taken.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::UnknownAlgorithm`] for a name the port does not
    /// compute.
    pub fn with_algorithm_name(
        algorithm_name: &str,
        source: &[u8],
        salt: Option<&[u8]>,
        iterations: u32,
    ) -> Result<Self, CryptoError> {
        let algorithm = HashAlgorithm::from_name(algorithm_name)
            .ok_or_else(|| CryptoError::UnknownAlgorithm(algorithm_name.to_string()))?;

        Ok(Self::new(algorithm, source, salt, iterations))
    }

    /// Returns the algorithm of the hash.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Returns how many times the digest was taken.
    pub fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Returns the raw bytes of the hash.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the hash in lowercase hexadecimal.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Returns the hash in standard base64.
    pub fn to_base64(&self) -> String {
        use base64::{Engine, engine::general_purpose::STANDARD};

        STANDARD.encode(&self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{HashAlgorithm, SimpleHash};

    #[test]
    fn the_digest_of_a_value_is_the_one_the_algorithm_gives() {
        let hello = b"hello";

        assert_eq!(
            SimpleHash::new(HashAlgorithm::Md5, hello, None, 1).to_hex(),
            "5d41402abc4b2a76b9719d911017c592"
        );
        assert_eq!(
            SimpleHash::new(HashAlgorithm::Sha1, hello, None, 1).to_hex(),
            "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
        );
        assert_eq!(
            SimpleHash::new(HashAlgorithm::Sha256, hello, None, 1).to_hex(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(
            SimpleHash::new(HashAlgorithm::Sha512, hello, None, 1).to_hex(),
            "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d9\
             9ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"
                .replace(['\n', ' '], "")
        );
    }

    #[test]
    fn a_salt_changes_the_hash_and_is_mixed_in_first() {
        let without_salt = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1);
        let with_salt = SimpleHash::new(HashAlgorithm::Sha256, b"secret", Some(b"pepper"), 1);

        assert_ne!(without_salt.bytes(), with_salt.bytes());

        // The salt is fed to the digest before the value, which is what Shiro's
        // `SimpleHash` does.
        let mut salted = b"pepper".to_vec();
        salted.extend_from_slice(b"secret");
        assert_eq!(
            with_salt.to_hex(),
            SimpleHash::new(HashAlgorithm::Sha256, &salted, None, 1).to_hex()
        );
    }

    #[test]
    fn more_iterations_digest_the_previous_digest() {
        let once = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1);
        let twice = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 2);

        assert_ne!(once.bytes(), twice.bytes());
        assert_eq!(
            twice.to_hex(),
            SimpleHash::new(HashAlgorithm::Sha256, once.bytes(), None, 1).to_hex()
        );
        assert_eq!(twice.iterations(), 2);
    }

    #[test]
    fn no_iterations_is_read_as_one() {
        let zero = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 0);
        let one = SimpleHash::new(HashAlgorithm::Sha256, b"secret", None, 1);

        assert_eq!(zero, one);
        assert_eq!(zero.iterations(), SimpleHash::DEFAULT_ITERATIONS);
    }

    #[test]
    fn an_algorithm_is_named_the_way_shiro_names_it() {
        assert_eq!(HashAlgorithm::from_name("sha-256"), Some(HashAlgorithm::Sha256));
        assert_eq!(HashAlgorithm::from_name("SHA512"), Some(HashAlgorithm::Sha512));
        assert_eq!(HashAlgorithm::from_name("crc32"), None);
        assert_eq!(HashAlgorithm::Sha256.name(), "SHA-256");

        assert!(SimpleHash::with_algorithm_name("crc32", b"x", None, 1).is_err());
    }

    #[test]
    fn the_hash_is_rendered_in_hex_and_base64() {
        let hash = SimpleHash::new(HashAlgorithm::Sha256, b"hello", None, 1);

        assert_eq!(hash.to_base64(), "LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=");
    }
}
