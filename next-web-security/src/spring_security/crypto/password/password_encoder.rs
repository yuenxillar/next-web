use next_web_core::error::BoxError;

/// Service interface for encoding passwords. The preferred implementation is BCryptPasswordEncoder.
pub trait PasswordEncoder
where
    Self: Send + Sync,
{
    /// Encode the raw password. Generally, a good encoding algorithm uses an adaptive
    /// one-way function.
    ///
    /// The value can be `None` in the event that the user has no password; in which
    /// case the result must be `None`.
    ///
    /// Returns a non-`None` encoded password, unless the `raw_password` was `None` in
    /// which case the result must be `None`.
    fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError>;

    /// Verify the encoded password obtained from storage matches the submitted raw
    /// password after it too is encoded. Returns `true` if the passwords match, `false`
    /// if they do not. The stored password itself is never decoded. Never `true` if
    /// either `raw_password` or `encoded_password` is `None` or an empty string.
    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool;

    /// Returns `true` if the encoded password should be encoded again for better
    /// security, else `false`. The default implementation always returns `false`.
    ///
    /// If `encoded_password` is `None` (the user didn't have a password), then
    /// always `false`.
    #[allow(unused_variables)]
    fn upgrade_encoding(&self, encoded_password: &str) -> Result<bool, BoxError> {
        Ok(false)
    }
}
