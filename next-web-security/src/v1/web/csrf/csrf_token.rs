/// Provides the information about an expected CSRF token.
pub trait CsrfToken
where
    Self: Send + Sync,
{
    ///
    /// Gets the HTTP header that the CSRF is populated on the response and can be placed
    /// on requests instead of the parameter. Cannot be null.
    ///
    /// #return
    ///
    /// the HTTP header that the CSRF is populated on the response and can be
    /// placed on requests instead of the parameter
    ///
    fn header_name(&self) -> &str;

    ////
    /// Gets the HTTP parameter name that should contain the token. Cannot be null.
    ///
    /// #return
    ///
    /// the HTTP parameter name that should contain the token.
    ///
    fn parameter_name(&self) -> &str;

    ////
    /// Gets the token value. Cannot be null.
    ///
    /// #return
    ///
    /// the token value
    ///
    fn token(&self) -> &str;
}
