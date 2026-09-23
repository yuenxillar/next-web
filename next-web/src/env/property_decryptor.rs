//! The strategies that decrypt the encrypted values of the configuration.

use next_web_core::error::BoxError;

/// The prefix that marks the value of a property as encrypted.
///
/// The prefix is the [`NEXT_SECURE_PROPERTIE`] marker followed by a colon, and
/// the part of the value that follows the colon is the ciphertext of the
/// property:
///
/// ```yaml
/// next:
///   data:
///     redis:
///       password: "nsp:BQ5S6j9Xt8Q0yQ6Z7t3YvA=="
/// ```
///
/// The value is decrypted while the environment of the application is prepared,
/// so the rest of the application only ever sees the plaintext, for example
/// through the `next.data.redis.password` property.
pub const SECURE_PROPERTY_PREFIX: &str = "nsp:";

/// Decrypts the encrypted values of the configuration properties.
///
/// A decryptor is the extension point of the configuration decryption: it is
/// applied to every property whose value starts with the
/// [`prefix`](Self::prefix) it declares, and it is given the part of the value
/// that follows that prefix. A property with the default
/// [`nsp:`][SECURE_PROPERTY_PREFIX] prefix is decrypted by the framework itself,
/// while an application adds its own algorithm with
/// [`submit_decryptor!`](crate::submit_decryptor):
///
/// ```ignore
/// struct MyDecryptor;
///
/// impl next_web::crypto::decrypt::PropertyDecryptor for MyDecryptor {
///     fn prefix(&self) -> &str {
///         "mycipher:"
///     }
///
///     fn decrypt(&self, ciphertext: &str, password: &str) -> Result<String, next_web::core::error::BoxError> {
///         // Decrypt the ciphertext, for example with the key that is specific
///         // to the application.
///         Ok(decrypt_with_my_algorithm(ciphertext, password))
///     }
/// }
///
/// next_web::submit_decryptor!(MyDecryptor);
/// ```
pub trait PropertyDecryptor
where
    Self: Send + Sync + 'static,
{
    /// Returns the prefix that marks the values this decryptor handles.
    ///
    /// The default is [`SECURE_PROPERTY_PREFIX`]. A value only reaches the
    /// decryptor when it starts with this prefix, so an empty prefix is ignored.
    fn prefix(&self) -> &str {
        SECURE_PROPERTY_PREFIX
    }

    /// Decrypts the ciphertext of the value of a property.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The value of the property without the prefix of the
    ///   decryptor, trimmed.
    /// * `password` - The password of the application, taken from the
    ///   [`DECRYPT_PASSWORD_PROPERTY`](super::DECRYPT_PASSWORD_PROPERTY)
    ///   property or from the [`NEXT_DECRYPT_PASSWORD`] environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error when the ciphertext cannot be decrypted. The error is
    /// reported with the property that could not be decrypted and the
    /// application stops, so that a wrong password or a corrupted value never
    /// reaches the application silently.
    fn decrypt(&self, ciphertext: &str, password: &str) -> Result<String, BoxError>;
}

inventory::collect!(&'static dyn PropertyDecryptor);

/// Registers the [`PropertyDecryptor`] of the given type.
///
/// Every property whose value starts with the prefix of the registered
/// decryptor is decrypted with it while the environment of the application is
/// prepared.
///
/// # Examples
///
/// ```ignore
/// struct MyDecryptor;
///
/// impl next_web::crypto::decrypt::PropertyDecryptor for MyDecryptor {
///     fn prefix(&self) -> &str {
///         "mycipher:"
///     }
///
///     fn decrypt(&self, ciphertext: &str, password: &str) -> Result<String, next_web::core::error::BoxError> {
///         Ok(ciphertext.to_owned())
///     }
/// }
///
/// next_web::submit_decryptor!(MyDecryptor);
/// ```
#[macro_export]
macro_rules! submit_decryptor {
    ($ty:ident) => {
        $crate::macros::submit! {
            &$ty as &dyn $crate::env::PropertyDecryptor
        }
    };
}

/// The [`PropertyDecryptor`] that decrypts the values of the
/// [`SECURE_PROPERTY_PREFIX`] with the AES-GCM algorithm of
/// [`crate::util::aes`].
///
/// It is the decryptor that the framework applies by default, and it needs the
/// `decrypt-properties` feature, which is enabled by default.
#[cfg(feature = "decrypt-properties")]
#[derive(Debug, Clone, Copy, Default)]
pub struct AesPropertyDecryptor;

#[cfg(feature = "decrypt-properties")]
impl PropertyDecryptor for AesPropertyDecryptor {
    fn decrypt(&self, ciphertext: &str, password: &str) -> Result<String, BoxError> {
        crate::util::aes::decrypt(ciphertext, password).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::constants::application_constants::NEXT_SECURE_PROPERTIE;

    use super::*;

    #[test]
    fn marks_the_values_with_the_secure_property_marker() {
        assert!(SECURE_PROPERTY_PREFIX.starts_with(NEXT_SECURE_PROPERTIE));
        assert_eq!(SECURE_PROPERTY_PREFIX, "nsp:");
    }

    #[test]
    fn the_default_prefix_of_a_decryptor_is_the_secure_property_prefix() {
        struct TestDecryptor;

        impl PropertyDecryptor for TestDecryptor {
            fn decrypt(&self, ciphertext: &str, _password: &str) -> Result<String, BoxError> {
                Ok(ciphertext.to_owned())
            }
        }

        assert_eq!(TestDecryptor.prefix(), SECURE_PROPERTY_PREFIX);
    }
}
