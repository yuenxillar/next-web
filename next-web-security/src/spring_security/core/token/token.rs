/// A token issued by TokenService.
/// It is important that the keys assigned to tokens are sufficiently randomised and secured
/// that they can serve as identifying a unique user session. Implementations of TokenService are free
/// to use encryption or encoding strategies of their choice. It is strongly recommended that keys
/// are of sufficient length to balance safety against persistence cost. In relation to persistence cost,
/// it is strongly recommended that returned keys are small enough for encoding in a cookie.
pub trait Token {
    /// Obtains the randomised, secure key assigned to this token. Presentation of this
    /// token to [`TokenService`] will always return a `Token` that is equal
    /// to the original `Token` issued for that key.
    ///
    /// # Returns
    /// A key with appropriate randomness and security.
    fn key(&self) -> &str;

    /// Returns the creation time as milliseconds since Unix epoch.
    ///
    /// # Returns
    /// The creation time in milliseconds since the Unix epoch.
    fn key_creation_time(&self) -> i64;

    /// Obtains the extended information associated within the token, which was presented
    /// when the token was first created.
    ///
    /// # Returns
    /// The user-specified extended information, if any.
    fn extended_information(&self) -> &str;
}
