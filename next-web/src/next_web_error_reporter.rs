/// Error reporter.
///
/// This trait is used to report errors that occur while the framework is
/// running. Implementors may log the error, forward it to a monitoring
/// system, write it to a file, or perform any other custom handling.
///
/// # Implementation notes
///
/// - Implementors should ensure that `report_error` does not panic on
///   failure, so that the main flow is not disrupted.
/// - The method takes `&mut self`, so implementors may keep mutable state,
///   such as an error counter, a buffer, or a connection handle.
/// - The error is passed as a trait object
///   (`&(dyn std::error::Error + 'static)`), so callers do not need to care
///   about the concrete error type. Implementors can obtain a textual
///   description through `Display` or `Debug`.
///
/// # Example
///
/// ```
/// use std::error::Error;
///
/// struct LogErrorReporter;
///
/// impl NextWebErrorReporter for LogErrorReporter {
///     fn report_error(&mut self, error: &(dyn Error + 'static)) {
///         eprintln!("[error] {error}");
///     }
/// }
/// ```
pub trait NextWebErrorReporter {
    /// Reports an error.
    ///
    /// # Arguments
    ///
    /// * `error` - A reference to the error to report. Its type is
    ///   `&(dyn std::error::Error + 'static)`, so methods such as
    ///   `to_string()` and `source()` can be called on it to obtain more
    ///   details.
    ///
    /// # Returns
    ///
    /// `true` if the failure was reported, or `false` if default reporting
    /// should occur.
    ///
    /// # Notes
    ///
    /// Implementations should try to ensure that this method itself does not
    /// fail. If an error occurs while reporting, it is recommended to swallow
    /// or degrade it internally rather than propagate it outward.
    fn report_error(&mut self, error: &(dyn std::error::Error + 'static)) -> bool;
}
