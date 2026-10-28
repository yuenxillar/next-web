use std::fmt;

use crate::autoconfigure::condition::ConditionMessage;

/// Outcome of a condition match, including a human-readable message.
///
/// A [`ConditionOutcome`] pairs a boolean match flag with a [`ConditionMessage`]
/// describing why the condition matched or did not match. It is the value type
/// returned by condition evaluation logic.
///
/// Construct instances through the [`ConditionOutcome::match_`] and
/// [`ConditionOutcome::no_match`] factory functions where possible, so that the
/// message is always built consistently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionOutcome {
    /// Whether the condition matched.
    matched: bool,
    /// The message describing the outcome.
    message: ConditionMessage,
}

impl ConditionOutcome {
    /// Creates a new outcome from a raw string message.
    ///
    /// For more consistent messages, prefer [`ConditionOutcome::with_message`].
    ///
    /// # Panics
    ///
    /// This function does not panic; the message is wrapped via
    /// [`ConditionMessage::of`].
    pub fn new(matched: bool, message: impl Into<String>) -> Self {
        Self::with_message(matched, ConditionMessage::of(message, &[]))
    }

    /// Creates a new outcome from a [`ConditionMessage`].
    pub fn with_message(matched: bool, message: ConditionMessage) -> Self {
        Self { matched, message }
    }

    /// Creates a new outcome representing a match with an empty message.
    pub fn match_() -> Self {
        Self::match_message(ConditionMessage::empty())
    }

    /// Creates a new outcome representing a match with a raw string message.
    ///
    /// For more consistent messages, prefer [`ConditionOutcome::match_message`].
    pub fn match_str(message: impl Into<String>) -> Self {
        Self::new(true, message)
    }

    /// Creates a new outcome representing a match with a [`ConditionMessage`].
    pub fn match_message(message: ConditionMessage) -> Self {
        Self::with_message(true, message)
    }

    /// Creates a new outcome representing no match with a raw string message.
    ///
    /// For more consistent messages, prefer [`ConditionOutcome::no_match_message`].
    pub fn no_match_str(message: impl Into<String>) -> Self {
        Self::new(false, message)
    }

    /// Creates a new outcome representing no match with a [`ConditionMessage`].
    pub fn no_match_message(message: ConditionMessage) -> Self {
        Self::with_message(false, message)
    }

    /// Returns `true` if the outcome was a match.
    pub fn is_match(&self) -> bool {
        self.matched
    }

    /// Returns the outcome message, or `None` if the message is empty.
    pub fn message(&self) -> Option<String> {
        if self.message.is_empty() {
            None
        } else {
            Some(self.message.to_string())
        }
    }

    /// Returns the underlying [`ConditionMessage`].
    pub fn condition_message(&self) -> &ConditionMessage {
        &self.message
    }
}

/// Defaults to the same string rendering as the outcome message.
impl fmt::Display for ConditionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Equivalent to the original `toString()`: render the message, or an
        // empty string when there is no message.
        match self.message() {
            Some(message) => f.write_str(&message),
            None => Ok(()),
        }
    }
}
