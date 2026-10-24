//! Error types used by the state machine runtime.
//!
//! The Java original throws `StateMachineException` and a handful of
//! `IllegalStateException`/`IllegalArgumentException` variants. Rust cannot
//! catch several inheritance unrelated exceptions, so every failure is
//! expressed as a [`StateMachineException`] carrying a
//! [`StateMachineErrorKind`] that callers can compare against.

use std::fmt::{Display, Formatter, Result as FmtResult};

use next_web_core::error::BoxError;

/// The kind of a [`StateMachineException`].
///
/// The kind is the part of the error that callers are expected to match on,
/// the message is only meant for logs and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StateMachineErrorKind {
    /// A transition, an action or a guard failed to execute.
    Execution,
    /// The state machine is already in an error state and rejects new events.
    StateMachineInErrorState,
    /// The state machine was used before it was started.
    NotStarted,
    /// A state referenced by a transition could not be resolved.
    StateNotFound,
    /// The configuration is malformed, for example a missing initial state.
    MalformedConfiguration,
    /// Security rules denied the operation.
    AccessDenied,
    /// The configuration asks for a feature that is not supported.
    Unsupported,
    /// No machine or persisted state exists for the requested identifier.
    MachineNotFound,
}

impl StateMachineErrorKind {
    /// Returns a short, stable description of the kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Execution => "execution error",
            Self::StateMachineInErrorState => "state machine is in error state",
            Self::NotStarted => "state machine has not been started",
            Self::StateNotFound => "state not found",
            Self::MalformedConfiguration => "malformed configuration",
            Self::AccessDenied => "access denied",
            Self::Unsupported => "unsupported feature",
            Self::MachineNotFound => "state machine not found",
        }
    }
}

/// The error returned by every state machine operation.
pub struct StateMachineException {
    kind: StateMachineErrorKind,
    message: String,
    source: Option<BoxError>,
}

impl StateMachineException {
    /// Creates a new error with the given kind and message.
    ///
    /// # Arguments
    /// * `kind` - the kind of the error
    /// * `message` - a human readable description
    pub fn new(kind: StateMachineErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    /// Attaches an underlying error.
    ///
    /// # Arguments
    /// * `source` - the error that caused this one
    pub fn with_source(mut self, source: BoxError) -> Self {
        self.source = Some(source);
        self
    }

    /// Gets the kind of this error.
    pub fn kind(&self) -> StateMachineErrorKind {
        self.kind
    }

    /// Gets the message of this error.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Checks whether this error has the given kind.
    ///
    /// # Arguments
    /// * `kind` - the kind to compare against
    pub fn is(&self, kind: StateMachineErrorKind) -> bool {
        self.kind == kind
    }
}

impl Display for StateMachineException {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::fmt::Debug for StateMachineException {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("StateMachineException")
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("source", &self.source.as_ref().map(ToString::to_string))
            .finish()
    }
}

impl std::error::Error for StateMachineException {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|source| source.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// Creates a [`StateMachineException`] of the given kind as a boxed error.
///
/// # Arguments
/// * `kind` - the kind of the error
/// * `message` - a human readable description
pub fn state_machine_error(kind: StateMachineErrorKind, message: impl Into<String>) -> BoxError {
    Box::new(StateMachineException::new(kind, message))
}

/// Creates an [`StateMachineErrorKind::Execution`] error as a boxed error.
///
/// # Arguments
/// * `message` - a human readable description
pub fn execution_error(message: impl Into<String>) -> BoxError {
    state_machine_error(StateMachineErrorKind::Execution, message)
}
