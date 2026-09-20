use axum::response::IntoResponse;

/// A trait for solving and transforming errors into appropriate response types
///
/// This trait provides a mechanism to convert error strings into structured response types
/// that can be returned from HTTP handlers. It allows for customization of error formatting
/// and header management.
///
/// # Type Parameters
///
/// * `T` - The response type that the error will be transformed into. Must implement:
///   - `serde::Serialize`      for serialization
///   - `Debug`                 for logging and display purposes
///   - `Send` and `Sync`       for thread safety
///   - `Clone`                 for potential clones
///   - `IntoResponse`          Used to generate the final return response
///
/// # Examples
///
/// ```
/// use axum::response::IntoResponse;
/// use your_crate::ErrorSolver;
///
/// // Using the default string implementation
/// let error = "Something went wrong".to_string();
/// let response = <() as ErrorSolver>::solve_error(error);
/// ```
pub trait ErrorHandler {
    /// Transforms an error string into an appropriate response type
    ///
    /// This method takes a raw error string and converts it into the desired response type `T`.
    ///
    /// # Parameters
    ///
    /// * `error`   - The error message to be transformed
    ///
    /// # Returns
    ///
    /// Returns the transformed error as type `T`, ready to be used as an HTTP response
    ///
    fn handle_error<'a>(error: &'a str) -> impl IntoResponse;
}
/// Default implementation of `ErrorSolver` for the unit type `()`
///
/// This implementation provides basic error handling that returns plain text responses
/// with appropriate content type headers.
impl ErrorHandler for () {
    /// Transforms an error string into a plain text response
    ///
    /// This implementation:
    /// - Returns the original error string as the response body
    ///
    /// # Parameters
    ///
    /// * `error` - The error message to be returned as plain text
    ///
    /// # Returns
    ///
    /// Returns the original error string unchanged
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::ErrorSolver;
    ///
    /// let error_message = "Database connection failed".to_string();
    /// let response = <() as ErrorSolver>::solve_error(error_message);
    ///
    /// println!("{:?}", response);
    /// ```
    fn handle_error<'a>(error: &'a str) -> impl IntoResponse {
        error.to_owned()
    }
}
