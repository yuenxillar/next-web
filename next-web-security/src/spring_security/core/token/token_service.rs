use std::sync::Arc;

use super::Token;

/// Provides a mechanism to allocate and rebuild secure, randomised tokens.
///
/// Implementations are solely concerned with issuing a new [`Token`] on demand. The
/// issued `Token` may contain user-specified extended information. The token
/// also contains a cryptographically strong, byte array-based key. This permits the token
/// to be used to identify a user session, if desired. The key can subsequently be
/// re-presented to the `TokenService` for verification and reconstruction of a
/// `Token` equal to the original `Token`.
///
/// Given the tightly-focused behaviour provided by this trait, it can serve as a
/// building block for more sophisticated token-based solutions. For example,
/// authentication systems that depend on stateless session keys. These could, for
/// instance, place the username inside the user-specified extended information associated
/// with the key. It is important to recognise that we do not intend for this trait to
/// be expanded to provide such capabilities directly.
pub trait TokenService: Send + Sync {
    /// Forces the allocation of a new [`Token`].
    ///
    /// # Arguments
    /// * `extended_information` - The extended information desired in the token
    ///   (cannot be empty, but can be an empty string)
    ///
    /// # Returns
    /// A new token that has not been issued previously, and is guaranteed to be
    /// recognised by this implementation's [`verify_token`] at any future time.
    ///
    fn allocate_token(&self, extended_information: &str) -> Arc<dyn Token>;

    /// Permits verification that the [`Token::key()`] was issued by this
    /// `TokenService` and reconstructs the corresponding `Token`.
    ///
    /// # Arguments
    /// * `key` - As obtained from [`Token::key()`] and created by this implementation
    ///
    /// # Returns
    /// The token, or `None` if the token was not issued by this
    /// `TokenService`.
    fn verify_token(&self, key: &str) -> Option<Arc<dyn Token>>;
}
