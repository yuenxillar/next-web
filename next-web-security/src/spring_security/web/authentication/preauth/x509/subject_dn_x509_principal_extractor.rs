use regex::RegexBuilder;

use crate::core::{AuthenticationError, AuthenticationErrorKind};

use super::{x509_certificate::X509Certificate, x509_principal_extractor::X509PrincipalExtractor};

/// Obtains the principal from a certificate using a regular expression match against the
/// subject distinguished name.
///
/// The regular expression should contain a single group. The default expression
/// `CN=(.*?)(?:,|$)` matches the common name field, so `CN=Jimi Hendrix, OU=...` yields a
/// user name of `Jimi Hendrix`. Matching is case insensitive.
#[deprecated(note = "Please use SubjectX500PrincipalExtractor instead")]
#[derive(Clone, Debug)]
#[allow(deprecated)]
pub struct SubjectDnX509PrincipalExtractor {
    pattern: regex::Regex,
}

#[allow(deprecated)]
impl SubjectDnX509PrincipalExtractor {
    /// Creates a new extractor using the default common name expression.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the regular expression used to extract the user name from the subject DN.
    ///
    /// # Panics
    ///
    /// Panics when the expression is empty or cannot be compiled. This is intended to be
    /// used during application configuration.
    pub fn set_subject_dn_regex(&mut self, subject_dn_regex: impl AsRef<str>) {
        let subject_dn_regex = subject_dn_regex.as_ref();
        assert!(
            !subject_dn_regex.trim().is_empty(),
            "Regular expression may not be null or empty"
        );
        self.pattern = RegexBuilder::new(subject_dn_regex)
            .case_insensitive(true)
            .build()
            .expect("subject DN regular expression must be valid");
    }
}

#[allow(deprecated)]
impl Default for SubjectDnX509PrincipalExtractor {
    fn default() -> Self {
        let pattern = RegexBuilder::new("CN=(.*?)(?:,|$)")
            .case_insensitive(true)
            .build()
            .expect("default subject DN regular expression must be valid");
        Self { pattern }
    }
}

#[allow(deprecated)]
impl X509PrincipalExtractor for SubjectDnX509PrincipalExtractor {
    fn extract_principal(
        &self,
        certificate: &X509Certificate,
    ) -> Result<String, AuthenticationError> {
        let subject_dn = certificate.subject_dn();
        let captures = self.pattern.captures(subject_dn).ok_or_else(|| {
            AuthenticationError::with_kind(
                format!(
                    "No matching pattern was found in subject DN: {}",
                    subject_dn
                ),
                AuthenticationErrorKind::BadCredentials,
            )
        })?;

        if captures.len() != 2 {
            return Err(AuthenticationError::with_kind(
                "Regular expression must contain a single group",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        captures
            .get(1)
            .map(|group| group.as_str().to_string())
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    format!(
                        "No matching pattern was found in subject DN: {}",
                        subject_dn
                    ),
                    AuthenticationErrorKind::BadCredentials,
                )
            })
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn extracts_common_name_with_default_expression() {
        let extractor = SubjectDnX509PrincipalExtractor::default();
        let certificate = X509Certificate::new("CN=Jimi Hendrix, OU=Musicians, O=Example");

        let principal = extractor
            .extract_principal(&certificate)
            .expect("principal should be extracted");

        assert_eq!(principal, "Jimi Hendrix");
    }

    #[test]
    fn matches_case_insensitively() {
        let mut extractor = SubjectDnX509PrincipalExtractor::default();
        extractor.set_subject_dn_regex("emailAddress=(.*?),");

        let certificate = X509Certificate::new("EMAILADDRESS=jimi@hendrix.org, CN=Jimi Hendrix");
        let principal = extractor
            .extract_principal(&certificate)
            .expect("principal should be extracted");

        assert_eq!(principal, "jimi@hendrix.org");
    }

    #[test]
    fn fails_when_no_match() {
        let extractor = SubjectDnX509PrincipalExtractor::default();
        let certificate = X509Certificate::new("O=Example, C=US");

        let error = extractor
            .extract_principal(&certificate)
            .expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }
}
