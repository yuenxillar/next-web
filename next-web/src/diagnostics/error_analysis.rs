use std::panic::PanicHookInfo;

/// The result of analyzing a error.
///
/// This struct encapsulates the outcome of a error analysis, providing
/// a human-readable description of what went wrong, an actionable suggestion
/// for resolving the problem, and the underlying cause of the error.
///
/// It is commonly used in diagnostic reporting, such as during application
/// startup errors or configuration errors.
///
/// # Examples
///
/// ```
/// use std::io;
///
/// use next_web::diagnostics::error_analysis::ErrorAnalysis;
///
/// let cause = io::Error::new(io::ErrorKind::NotFound, "redis.conf not found");
/// let analysis = ErrorAnalysis::new(
///     "Failed to load Redis configuration file",
///     Some("Ensure that redis.conf exists in the config directory"),
///     Some(&cause),
/// );
///
/// println!("Description: {}", analysis.description());
/// println!("Action: {}", analysis.action().unwrap());
/// println!("Cause: {}", analysis.cause().unwrap());
/// ```
#[derive(Debug)]
pub struct ErrorAnalysis<'a> {
    /// A human-readable description of the error.
    description: String,

    /// An actionable suggestion for addressing the error, if any.
    action: Option<String>,

    /// The underlying cause of the error, if available.
    ///
    /// The error is borrowed for as long as this analysis lives, so callers can
    /// inspect the original error through [`Self::cause`] without giving up
    /// ownership.
    cause: Option<&'a (dyn std::error::Error + 'static)>,
}

impl<'a> ErrorAnalysis<'a> {
    /// Creates a new `ErrorAnalysis` with the given description, action, and cause.
    ///
    /// # Arguments
    ///
    /// * `description` - A human-readable description of the error
    /// * `action` - An optional action that the user should take to address the problem
    /// * `cause` - An optional underlying cause of the error
    ///
    /// # Examples
    ///
    /// ```
    /// use next_web::diagnostics::error_analysis::ErrorAnalysis;
    ///
    /// let analysis = ErrorAnalysis::new(
    ///     "Connection refused",
    ///     Some("Check if the Redis server is running".to_string()),
    ///     None,
    /// );
    /// ```
    pub fn new(
        description: impl Into<String>,
        action: Option<impl Into<String>>,
        cause: Option<&'a (dyn std::error::Error + 'static)>,
    ) -> Self {
        Self {
            description: description.into(),
            action: action.map(Into::into),
            cause,
        }
    }

    /// Returns a description of the error.
    ///
    /// # Returns
    ///
    /// A string slice containing the description of what went wrong.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the action, if any, to be taken to address the error.
    ///
    /// # Returns
    ///
    /// An optional string slice containing the suggested action, or `None`
    /// if no specific action is available.
    pub fn action(&self) -> Option<&str> {
        self.action.as_deref()
    }

    /// Returns the cause of the error, if available.
    ///
    /// # Returns
    ///
    /// An optional reference to the underlying error that caused the error.
    pub fn cause(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
    }

    /// Sets the action for this error analysis.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to be taken to address the error
    ///
    /// # Returns
    ///
    /// A mutable reference to `self` for method chaining.
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Sets the cause for this error analysis.
    ///
    /// # Arguments
    ///
    /// * `cause` - The underlying cause of the error
    ///
    /// # Returns
    ///
    /// A mutable reference to `self` for method chaining.
    pub fn with_cause(mut self, cause: &'a (dyn std::error::Error + 'static)) -> Self {
        self.cause = Some(cause);
        self
    }
}

impl<'a> ErrorAnalysis<'a> {
    pub fn with_panic_hook(hook: &PanicHookInfo<'a>) -> Self {
        let message = hook.payload_as_str().unwrap_or("Unknown message");

        let location = hook
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let description = format!(
            "Location:\n[{}]\n\nError:\n{}\n\n\
         General advice:\n\
         - Avoid using unwrap() in production code\n\
         - Use ? operator to propagate errors\n\
         - Use match for explicit error handling\n\
         - Use unwrap_or() or unwrap_or_else() with fallback values\n\
         - If truly unrecoverable, use expect() with a clear error message\n",
            location, message,
        );

        Self {
            description,
            action: None,
            cause: None,
        }
    }
}

impl<'a> std::fmt::Display for ErrorAnalysis<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Description: {}", self.description)?;
        if let Some(action) = &self.action {
            writeln!(f, "Action: {}", action)?;
        }
        if let Some(cause) = &self.cause {
            writeln!(f, "Cause: {}", cause)?;
        }
        Ok(())
    }
}

impl<'a> std::error::Error for ErrorAnalysis<'a> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
    }
}

impl<'a> From<String> for ErrorAnalysis<'a> {
    fn from(description: String) -> Self {
        ErrorAnalysis {
            description,
            action: None,
            cause: None,
        }
    }
}

impl<'a> From<&str> for ErrorAnalysis<'a> {
    fn from(description: &str) -> Self {
        description.to_string().into()
    }
}
