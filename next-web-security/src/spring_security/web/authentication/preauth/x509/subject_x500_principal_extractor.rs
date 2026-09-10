use crate::core::{AuthenticationError, AuthenticationErrorKind};

use super::{x509_certificate::X509Certificate, x509_principal_extractor::X509PrincipalExtractor};

/// Extracts the principal from the subject distinguished name of a certificate.
///
/// By default the common name (`CN`) attribute is used. When configured to extract the
/// principal from the email address, the `emailAddress` attribute (represented by the
/// `OID.1.2.840.113549.1.9.1` attribute type) is used instead.
///
/// Distinguished names are read from the most specific attribute to the least specific
/// one, so the left-most matching attribute wins. This mirrors the behaviour required
/// for certificates that contain more than one common name.
#[derive(Clone, Debug)]
pub struct SubjectX500PrincipalExtractor {
    extract_principal_name_from_email: bool,
}

impl SubjectX500PrincipalExtractor {
    /// Attribute type used for the common name.
    pub const CN_SUBJECT_DN_TYPE: &'static str = "CN";

    /// Attribute type used for the email address.
    pub const EMAIL_SUBJECT_DN_TYPE: &'static str = "OID.1.2.840.113549.1.9.1";

    /// Creates a new extractor that reads the common name.
    pub fn new() -> Self {
        Self {
            extract_principal_name_from_email: false,
        }
    }

    /// Sets whether the principal should be extracted from the email address instead of
    /// the common name.
    pub fn set_extract_principal_name_from_email(&mut self, extract: bool) {
        self.extract_principal_name_from_email = extract;
    }

    /// Returns whether the principal is extracted from the email address.
    pub fn is_extract_principal_name_from_email(&self) -> bool {
        self.extract_principal_name_from_email
    }

    fn matches(&self, attribute_type: &str) -> bool {
        if self.extract_principal_name_from_email {
            attribute_type.eq_ignore_ascii_case(Self::EMAIL_SUBJECT_DN_TYPE)
                || attribute_type.eq_ignore_ascii_case("EMAILADDRESS")
                || attribute_type.eq_ignore_ascii_case("E")
        } else {
            attribute_type.eq_ignore_ascii_case(Self::CN_SUBJECT_DN_TYPE)
        }
    }
}

impl Default for SubjectX500PrincipalExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl X509PrincipalExtractor for SubjectX500PrincipalExtractor {
    fn extract_principal(
        &self,
        certificate: &X509Certificate,
    ) -> Result<String, AuthenticationError> {
        let subject_dn = certificate.subject_dn();
        for (attribute_type, value) in parse_subject_dn(subject_dn) {
            if self.matches(&attribute_type) {
                return Ok(value);
            }
        }

        Err(AuthenticationError::with_kind(
            format!(
                "No matching pattern was found in subject DN: {}",
                subject_dn
            ),
            AuthenticationErrorKind::BadCredentials,
        ))
    }
}

/// Parses a distinguished name into its attribute type/value pairs, preserving the order
/// in which they appear.
fn parse_subject_dn(subject_dn: &str) -> Vec<(String, String)> {
    split_rdns(subject_dn)
        .into_iter()
        .filter_map(|rdn| parse_rdn(&rdn))
        .collect()
}

fn split_rdns(subject_dn: &str) -> Vec<String> {
    let mut rdns = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    let mut quoted = false;

    for ch in subject_dn.chars() {
        if escaped {
            current.push('\\');
            current.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => {
                quoted = !quoted;
                current.push(ch);
            }
            ',' if !quoted => {
                rdns.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        rdns.push(current);
    }

    rdns
}

fn parse_rdn(rdn: &str) -> Option<(String, String)> {
    let (attribute_type, value) = split_unescaped_once(rdn, '=')?;
    let attribute_type = attribute_type.trim();
    if attribute_type.is_empty() {
        return None;
    }
    Some((attribute_type.to_string(), unescape(value.trim())))
}

fn split_unescaped_once(value: &str, separator: char) -> Option<(String, String)> {
    let mut escaped = false;
    let mut quoted = false;

    for (index, ch) in value.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            c if c == separator && !quoted => {
                let head = value[..index].to_string();
                let tail = value[index + ch.len_utf8()..].to_string();
                return Some((head, tail));
            }
            _ => {}
        }
    }

    None
}

fn unescape(value: &str) -> String {
    let value = value.trim();
    let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    };

    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                result.push(escaped);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(subject_dn: &str) -> Result<String, AuthenticationError> {
        SubjectX500PrincipalExtractor::new().extract_principal(&X509Certificate::new(subject_dn))
    }

    #[test]
    fn extracts_common_name() {
        let principal = extract(
            "CN=Luke Taylor,OU=Open Source Development Lab.,O=Monkey Machine Ltd,L=Glasgow,ST=Scotland,C=UK",
        )
        .expect("principal should be extracted");

        assert_eq!(principal, "Luke Taylor");
    }

    #[test]
    fn extracts_most_specific_common_name() {
        let principal =
            extract("CN=alice,CN=bob,O=Example Corp,C=US").expect("principal should be extracted");

        assert_eq!(principal, "alice");
    }

    #[test]
    fn handles_common_name_at_the_end() {
        let principal =
            extract("L=Cupertino,C=US,ST=CA,OU=Java Software,O=Sun Microsystems\\, Inc,CN=Duke")
                .expect("principal should be extracted");

        assert_eq!(principal, "Duke");
    }

    #[test]
    fn ignores_embedded_distinguished_names() {
        let principal = extract("CN=luke,OU=CN=duke\\,,O=Example Corp,C=US")
            .expect("principal should be extracted");

        assert_eq!(principal, "luke");
    }

    #[test]
    fn extracts_email_address() {
        let mut extractor = SubjectX500PrincipalExtractor::new();
        extractor.set_extract_principal_name_from_email(true);

        let certificate = X509Certificate::new(
            "EMAILADDRESS=luke@monkeymachine,OU=OID.1.2.840.113549.1.9.1=duke@gorillagadget,O=Example Corp,C=US",
        );
        let principal = extractor
            .extract_principal(&certificate)
            .expect("principal should be extracted");

        assert_eq!(principal, "luke@monkeymachine");
    }

    #[test]
    fn fails_when_no_attribute_matches() {
        let error = extract("O=Example Corp,C=US").expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }
}
