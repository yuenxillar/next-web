pub mod allow_all_credentials_matcher;
pub mod credentials_matcher;
pub mod hashed_credentials_matcher;
pub mod password_matcher;
pub mod sha_credentials_matchers;
pub mod simple_credentials_matcher;

pub use allow_all_credentials_matcher::AllowAllCredentialsMatcher;
pub use credentials_matcher::CredentialsMatcher;
pub use hashed_credentials_matcher::HashedCredentialsMatcher;
pub use password_matcher::PasswordMatcher;
pub use sha_credentials_matchers::{Sha256CredentialsMatcher, Sha384CredentialsMatcher, Sha512CredentialsMatcher};
pub use simple_credentials_matcher::SimpleCredentialsMatcher;

/// Compares a submitted credential with a stored one without leaking where they
/// differ.
///
/// # Arguments
///
/// * `submitted` - The credential the user submitted, rendered as text.
/// * `stored` - The credential the store holds, rendered as text.
pub fn credentials_equal(submitted: &str, stored: &str) -> bool {
    let submitted = submitted.as_bytes();
    let stored = stored.as_bytes();

    if submitted.len() != stored.len() {
        return false;
    }

    let mut difference = 0u8;
    for (left, right) in submitted.iter().zip(stored.iter()) {
        difference |= left ^ right;
    }

    difference == 0
}

/// Returns the text of a credential.
///
/// # Arguments
///
/// * `credentials` - The credential as the object a token or an account holds.
pub fn credentials_text(credentials: &crate::core::util::object::Object) -> String {
    credentials.to_string()
}

#[cfg(test)]
mod tests {
    use super::{credentials_equal, credentials_text};
    use crate::core::util::object::Object;

    #[test]
    fn the_same_text_matches_and_a_different_one_does_not() {
        assert!(credentials_equal("secret", "secret"));
        assert!(!credentials_equal("secret", "Secret"));
        assert!(!credentials_equal("secret", "secret "));
        assert!(!credentials_equal("", "x"));
        assert!(credentials_equal("", ""));
    }

    #[test]
    fn a_credential_is_compared_as_the_text_it_renders_as() {
        assert_eq!(credentials_text(&Object::Str("secret".to_string())), "secret");
        assert_eq!(credentials_text(&Object::Int(42)), "42");
    }
}
