use std::panic::PanicHookInfo;

/// The result of analyzing a failure.
///
/// This struct encapsulates the outcome of a failure analysis, providing
/// a human-readable description of what went wrong, an actionable suggestion
/// for resolving the problem, and the underlying cause of the failure.
///
/// It is commonly used in diagnostic reporting, such as during application
/// startup failures or configuration errors.
///
/// # Examples
///
/// ```
/// use std::io;
///
/// let cause = io::Error::new(io::ErrorKind::NotFound, "redis.conf not found");
/// let analysis = FailureAnalysis::new(
///     "Failed to load Redis configuration file",
///     "Ensure that redis.conf exists in the config directory",
///     Some(Box::new(cause)),
/// );
///
/// println!("Description: {}", analysis.description());
/// println!("Action: {}", analysis.action().unwrap());
/// println!("Cause: {}", analysis.cause().unwrap());
/// ```
#[derive(Debug)]
pub struct FailureAnalysis {
    /// A human-readable description of the failure.
    description: String,

    /// An actionable suggestion for addressing the failure, if any.
    action: Option<String>,

    /// The underlying cause of the failure, if available.
    cause: Option<Box<dyn std::error::Error>>,
}

impl FailureAnalysis {
    /// Creates a new `FailureAnalysis` with the given description, action, and cause.
    ///
    /// # Arguments
    ///
    /// * `description` - A human-readable description of the failure
    /// * `action` - An optional action that the user should take to address the problem
    /// * `cause` - An optional underlying cause of the failure
    ///
    /// # Examples
    ///
    /// ```
    /// let analysis = FailureAnalysis::new(
    ///     "Connection refused",
    ///     Some("Check if the Redis server is running".to_string()),
    ///     None,
    /// );
    /// ```
    pub fn new(
        description: impl Into<String>,
        action: Option<impl Into<String>>,
        cause: Option<Box<dyn std::error::Error>>,
    ) -> Self {
        Self {
            description: description.into(),
            action: action.map(Into::into),
            cause,
        }
    }

    /// Returns a description of the failure.
    ///
    /// # Returns
    ///
    /// A string slice containing the description of what went wrong.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the action, if any, to be taken to address the failure.
    ///
    /// # Returns
    ///
    /// An optional string slice containing the suggested action, or `None`
    /// if no specific action is available.
    pub fn action(&self) -> Option<&str> {
        self.action.as_deref()
    }

    /// Returns the cause of the failure, if available.
    ///
    /// # Returns
    ///
    /// An optional reference to the underlying error that caused the failure.
    pub fn cause(&self) -> Option<&dyn std::error::Error> {
        self.cause.as_deref().map(|e| e as _)
    }

    /// Consumes the `FailureAnalysis` and returns the underlying cause, if any.
    ///
    /// # Returns
    ///
    /// An optional boxed error representing the cause of the failure.
    pub fn into_cause(self) -> Option<Box<dyn std::error::Error>> {
        self.cause
    }

    /// Sets the action for this failure analysis.
    ///
    /// # Arguments
    ///
    /// * `action` - The action to be taken to address the failure
    ///
    /// # Returns
    ///
    /// A mutable reference to `self` for method chaining.
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Sets the cause for this failure analysis.
    ///
    /// # Arguments
    ///
    /// * `cause` - The underlying cause of the failure
    ///
    /// # Returns
    ///
    /// A mutable reference to `self` for method chaining.
    pub fn with_cause(mut self, cause: Box<dyn std::error::Error + Send + Sync>) -> Self {
        self.cause = Some(cause);
        self
    }
}

impl FailureAnalysis {
    pub fn with_panic_hook<'a>(hook: &PanicHookInfo<'a>) -> Self {
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

impl std::fmt::Display for FailureAnalysis {
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

impl std::error::Error for FailureAnalysis {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

impl Into<FailureAnalysis> for String {
    fn into(self) -> FailureAnalysis {
        FailureAnalysis {
            description: self,
            action: None,
            cause: None,
        }
    }
}

impl Into<FailureAnalysis> for &str {
    fn into(self) -> FailureAnalysis {
        self.to_string().into()
    }
}

impl Into<FailureAnalysis> for Box<dyn std::error::Error> {
    fn into(self) -> FailureAnalysis {
        FailureAnalysis {
            description: self.to_string(),
            action: None,
            cause: Some(self),
        }
    }
}
