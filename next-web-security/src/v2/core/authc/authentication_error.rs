use std::{error::Error, fmt::Display};

/// Why a login did not happen.
///
/// Apache Shiro models these as a hierarchy of exceptions (`AuthenticationException`,
/// `IncorrectCredentialsException`, `LockedAccountException`, …); the port keeps
/// the cases a handler needs to tell apart in one enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationError {
    /// The submitted credentials are not the stored ones.
    InvalidCredentials,
    /// The account is locked and cannot log in.
    AccountLocked,
    /// No realm knows the token that was submitted.
    Unknown,
    /// A part of the port that had to answer is not implemented.
    NotImplemented(String),
    /// A refusal with the reason the caller should see.
    Custom(String),
}

impl AuthenticationError {
    /// Creates a refusal with the given reason.
    ///
    /// # Arguments
    ///
    /// * `message` - The reason the login was refused.
    pub fn custom(message: impl ToString) -> Self {
        Self::Custom(message.to_string())
    }

    /// Returns the reason of the refusal.
    pub fn message(&self) -> String {
        match self {
            Self::InvalidCredentials => {
                "The submitted credentials did not match the stored ones.".to_string()
            }
            Self::AccountLocked => "The account is locked.".to_string(),
            Self::Unknown => "No realm knows the token that was submitted.".to_string(),
            Self::NotImplemented(message) | Self::Custom(message) => message.clone(),
        }
    }
}

impl Error for AuthenticationError {}

impl Display for AuthenticationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => {
                write!(f, "The submitted credentials did not match the stored ones.")
            }
            Self::AccountLocked => write!(f, "The account is locked."),
            Self::Unknown => write!(f, "No realm knows the token that was submitted."),
            Self::NotImplemented(message) => write!(f, "Not implemented: {message}"),
            Self::Custom(message) => write!(f, "{message}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AuthenticationError;

    #[test]
    fn every_case_says_what_went_wrong() {
        assert_eq!(
            AuthenticationError::InvalidCredentials.to_string(),
            "The submitted credentials did not match the stored ones."
        );
        assert_eq!(
            AuthenticationError::AccountLocked.to_string(),
            "The account is locked."
        );
        assert!(AuthenticationError::Unknown.to_string().contains("No realm"));
        assert_eq!(
            AuthenticationError::NotImplemented("pam".to_string()).to_string(),
            "Not implemented: pam"
        );
        assert_eq!(AuthenticationError::custom("nope").to_string(), "nope");
        assert_eq!(AuthenticationError::custom("nope").message(), "nope");
    }
}
