use crate::crypto::password::PasswordEncoder;
use bcrypt::{hash, verify};
use next_web_core::error::BoxError;
use regex::Regex;

/// Minimum allowed BCrypt log rounds.
const MIN_LOG_ROUNDS: i32 = 4;
/// Maximum allowed BCrypt log rounds.
const MAX_LOG_ROUNDS: i32 = 31;
/// Sentinel value meaning "use the default strength".
const DEFAULT_STRENGTH: i32 = -1;
/// The strength applied when none is configured.
const FALLBACK_STRENGTH: u32 = 10;

/// Implementation of [`PasswordEncoder`] that uses the BCrypt strong hashing function.
///
/// Mirrors Spring Security's `BCryptPasswordEncoder` but does not support switching
/// between the `$2a` / `$2y` / `$2b` version prefixes on generation. Existing hashes
/// with any of those prefixes are still accepted during verification.
///
/// The default strength is 10. The larger the strength, the more work is required
/// (exponentially) to hash a password.
#[derive(Clone)]
pub struct BCryptPasswordEncoder {
    /// Log rounds, between 4 and 31.
    strength: u32,
    /// Pre-compiled pattern used to validate that an encoded password looks like BCrypt.
    bcrypt_pattern: Regex,
}

impl Default for BCryptPasswordEncoder {
    fn default() -> Self {
        Self::new(DEFAULT_STRENGTH)
    }
}

impl BCryptPasswordEncoder {
    /// Creates a `BCryptPasswordEncoder`.
    ///
    /// Passing `-1` for `strength` selects the default of 10. Any other value must
    /// fall within 4..=31, otherwise this constructor panics (matching the Java
    /// `IllegalArgumentException("Bad strength")`).
    pub fn new(strength: i32) -> Self {
        if strength != DEFAULT_STRENGTH && !(MIN_LOG_ROUNDS..=MAX_LOG_ROUNDS).contains(&strength) {
            panic!("Bad strength");
        }
        let strength = if strength == DEFAULT_STRENGTH {
            FALLBACK_STRENGTH
        } else {
            strength as u32
        };
        Self {
            strength,
            // Rust's regex crate anchors `^` and `$` to the whole text by default,
            // which is equivalent to Java's `\A` and `\z`.
            bcrypt_pattern: Regex::new(r"^\$2(a|y|b)?\$(\d\d)\$[./0-9A-Za-z]{53}$")
                .expect("BCrypt regex should be valid"),
        }
    }

    /// Returns the configured strength (log rounds).
    pub fn strength(&self) -> u32 {
        self.strength
    }

    /// Returns `true` if the given encoded password should be encoded again for
    /// better security.
    ///
    /// This is the case if and only if the cost recorded in the hash is lower than
    /// the currently configured cost.
    ///
    /// Panics if `encoded_password` is not a valid BCrypt hash (matching the Java
    /// `IllegalArgumentException`).
    pub fn upgrade_encoding_checked(&self, encoded_password: &str) -> bool {
        let caps = self
            .bcrypt_pattern
            .captures(encoded_password)
            .unwrap_or_else(|| {
                panic!("Encoded password does not look like BCrypt: {encoded_password}")
            });
        let strength: u32 = caps[2].parse().expect("regex guarantees two digits");
        strength < self.strength
    }
}

impl PasswordEncoder for BCryptPasswordEncoder {
    fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
        match raw_password {
            None => Ok(None),
            Some(password) => {
                // `bcrypt::hash` generates a random salt internally and returns a `$2b$` hash.
                let encoded = hash(password, self.strength)?;
                Ok(Some(encoded))
            }
        }
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        if !self.bcrypt_pattern.is_match(encoded_password) {
            // Corresponds to Java's logger.warn("Encoded password does not look like BCrypt").
            // No logging dependency is introduced here; callers decide how to record it.
            return false;
        }
        verify(raw_password, encoded_password).unwrap_or(false)
    }

    /// Returns `true` if the encoded password should be re-encoded.
    ///
    /// Unlike the Java version, this does **not** panic on invalid input: it returns
    /// `false` instead, because panicking inside a trait method would surprise
    /// callers. Use `upgrade_encoding_checked` if the strict Java behavior is needed.
    fn upgrade_encoding(&self, encoded_password: &str) -> Result<bool, BoxError> {
        let caps = self
            .bcrypt_pattern
            .captures(encoded_password)
            .ok_or_else(|| {
                format!("Encoded password does not look like BCrypt: {encoded_password}")
            })?;

        // The regex guarantees group 2 is exactly two ASCII digits, so this parse
        // cannot fail in practice.
        let strength: u32 = caps[2].parse().expect("regex guarantees two digits");

        Ok(strength < self.strength)
    }
}
