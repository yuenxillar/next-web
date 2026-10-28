use crate::authentication::password::CompromisedPasswordDecision;

/// An API for checking if a password has been compromised.
pub trait CompromisedPasswordChecker: Send + Sync {
    /// Check whether the password is compromised. If password is none,
    /// then the return value must be false for CompromisedPasswordDecision.is_compromised() since a none password represents no password (e.g. the user leverages Passkeys instead).
    fn check(&self, password: Option<&str>) -> CompromisedPasswordDecision;
}
