use std::sync::Arc;

use next_web_context::{support::MessageSourceAccessor, MessageSource};
use tracing::debug;

use crate::core::{AuthenticationError, AuthenticationErrorKind, NextSecurityMessageSource};

use super::{x509_certificate::X509Certificate, x509_principal_extractor::X509PrincipalExtractor};

/// Extracts the principal from the subject distinguished name of a certificate.
///
/// Depending on [`Self::set_extract_principal_name_from_email`], the principal is read
/// either from the common name (`CN`) attribute (the default) or from the
/// `emailAddress` attribute, identified by the `OID.1.2.840.113549.1.9.1` attribute type.
///
/// Distinguished names are read from the most specific attribute to the least specific
/// one, so the left-most matching attribute wins. This mirrors the behaviour required for
/// certificates that contain more than one common name.
#[derive(Clone)]
pub struct SubjectX500PrincipalExtractor {
    messages: MessageSourceAccessor,
    subject_dn_type: String,
}

impl SubjectX500PrincipalExtractor {
    /// Attribute type used for the common name.
    const CN_SUBJECT_DN_TYPE: &'static str = "CN";

    /// Attribute type used for the email address.
    const EMAIL_SUBJECT_DN_TYPE: &'static str = "OID.1.2.840.113549.1.9.1";

    /// Message code used when no attribute matches the configured subject DN type.
    const NO_MATCHING_MESSAGE_CODE: &'static str = "SubjectX500PrincipalExtractor.noMatching";

    /// Creates a new extractor that reads the common name.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the principal should be extracted from the email address instead of
    /// the common name.
    ///
    /// By default the principal is extracted from the `CN` attribute. When
    /// `extract_principal_name_from_email` is `true`, the principal is extracted from
    /// the email address, identified by the `OID.1.2.840.113549.1.9.1` attribute type.
    pub fn set_extract_principal_name_from_email(
        &mut self,
        extract_principal_name_from_email: bool,
    ) {
        if extract_principal_name_from_email {
            self.subject_dn_type = Self::EMAIL_SUBJECT_DN_TYPE.to_string();
        } else {
            self.subject_dn_type = Self::CN_SUBJECT_DN_TYPE.to_string();
        }
    }

    /// Returns whether the principal is extracted from the email address.
    pub fn is_extract_principal_name_from_email(&self) -> bool {
        self.subject_dn_type == Self::EMAIL_SUBJECT_DN_TYPE
    }

    /// Sets the message source to use for resolving error messages.
    pub fn set_message_source(&mut self, message_source: Arc<dyn MessageSource>) {
        self.messages = MessageSourceAccessor::new(message_source);
    }

    fn get_subject(&self, subject_dn: &str) -> Result<String, AuthenticationError> {
        for (attribute_type, value) in parse_subject_dn(subject_dn) {
            if self.subject_dn_type == attribute_type {
                return Ok(value);
            }
        }

        let message = self.messages.message_or_default(
            Self::NO_MATCHING_MESSAGE_CODE,
            None,
            &format!(
                "No matching pattern was found in subject DN: {}",
                subject_dn
            ),
        );

        Err(AuthenticationError::with_kind(
            message,
            AuthenticationErrorKind::BadCredentials,
        ))
    }
}

impl Default for SubjectX500PrincipalExtractor {
    fn default() -> Self {
        Self {
            messages: NextSecurityMessageSource::get_accessor(),
            subject_dn_type: Self::CN_SUBJECT_DN_TYPE.to_string(),
        }
    }
}

impl X509PrincipalExtractor for SubjectX500PrincipalExtractor {
    fn extract_principal(
        &self,
        certificate: &X509Certificate,
    ) -> Result<String, AuthenticationError> {
        let subject_dn = certificate.subject_dn();
        debug!("Subject DN is '{}'", subject_dn);
        let principal_name = self.get_subject(subject_dn)?;
        debug!("Extracted Principal name is '{}'", principal_name);
        Ok(principal_name)
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

/// Splits a distinguished name into relative distinguished names on unescaped, unquoted
/// commas.
fn split_rdns(subject_dn: &str) -> Vec<String> {
    let mut rdns = Vec::new();
    let mut current = String::new();
    let mut chars = subject_dn.chars().peekable();
    let mut quoted = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                current.push(ch);
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
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

/// Parses a single RDN. Multi-valued RDNs (containing an unescaped `+`) are ignored, just
/// like `Rdn.getType()` returns `null` for them.
fn parse_rdn(rdn: &str) -> Option<(String, String)> {
    if contains_top_level(rdn, '+') {
        return None;
    }

    let (attribute_type, value) = split_unescaped_once(rdn, '=')?;
    let attribute_type = attribute_type.trim();
    if attribute_type.is_empty() {
        return None;
    }

    Some((attribute_type.to_string(), decode_value(value.trim())))
}

/// Returns `true` when the given separator occurs at the top level, i.e. not escaped and
/// not inside a quoted string.
fn contains_top_level(value: &str, separator: char) -> bool {
    let mut escaped = false;
    let mut quoted = false;

    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            c if c == separator && !quoted => return true,
            _ => {}
        }
    }

    false
}

/// Splits the value on the first unescaped, unquoted occurrence of the given separator.
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

/// Decodes an RFC2253 attribute value.
///
/// A value starting with `#` is treated as a hex-encoded value; quoted values are
/// unquoted; and backslash escapes (including `\XX` hex pairs) are resolved.
fn decode_value(value: &str) -> String {
    if let Some(hex) = value.strip_prefix('#') {
        return decode_hex_string(hex);
    }

    let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    };

    unescape(value)
}

/// Decodes a hex string into a UTF-8 string, replacing invalid sequences.
fn decode_hex_string(hex: &str) -> String {
    let digits: Vec<char> = hex.chars().filter(|ch| !ch.is_whitespace()).collect();
    let mut bytes = Vec::with_capacity(digits.len() / 2);

    let mut index = 0;
    while index + 1 < digits.len() {
        match (digits[index].to_digit(16), digits[index + 1].to_digit(16)) {
            (Some(high), Some(low)) => bytes.push(((high << 4) | low) as u8),
            _ => {}
        }
        index += 2;
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

/// Resolves backslash escapes, including `\XX` hex pairs which represent raw bytes of the
/// UTF-8 encoded value.
fn unescape(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut bytes = Vec::with_capacity(value.len());
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '\\' && index + 1 < chars.len() {
            if index + 2 < chars.len() {
                if let (Some(high), Some(low)) =
                    (chars[index + 1].to_digit(16), chars[index + 2].to_digit(16))
                {
                    bytes.push(((high << 4) | low) as u8);
                    index += 3;
                    continue;
                }
            }

            push_char(&mut bytes, chars[index + 1]);
            index += 2;
            continue;
        }

        push_char(&mut bytes, ch);
        index += 1;
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

fn push_char(bytes: &mut Vec<u8>, ch: char) {
    let mut buffer = [0u8; 4];
    bytes.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
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
    fn common_name_matching_is_case_sensitive() {
        let error = extract("cn=alice,O=Example Corp,C=US").expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }

    #[test]
    fn ignores_multi_valued_rdn() {
        let error =
            extract("CN=alice+OU=Dev,O=Example Corp,C=US").expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }

    #[test]
    fn handles_quoted_value() {
        let principal =
            extract("CN=\"Duke, Esq\",O=Example Corp,C=US").expect("principal should be extracted");

        assert_eq!(principal, "Duke, Esq");
    }

    #[test]
    fn handles_hex_escaped_value() {
        let principal =
            extract("CN=Duke\\2C Esq,O=Example Corp,C=US").expect("principal should be extracted");

        assert_eq!(principal, "Duke, Esq");
    }

    #[test]
    fn handles_hex_encoded_value() {
        let principal =
            extract("CN=#4869,O=Example Corp,C=US").expect("principal should be extracted");

        assert_eq!(principal, "Hi");
    }

    #[test]
    fn extracts_email_address() {
        let mut extractor = SubjectX500PrincipalExtractor::new();
        extractor.set_extract_principal_name_from_email(true);

        let certificate = X509Certificate::new(
            "OID.1.2.840.113549.1.9.1=luke@monkeymachine,OU=OID.1.2.840.113549.1.9.1=duke@gorillagadget,O=Example Corp,C=US",
        );
        let principal = extractor
            .extract_principal(&certificate)
            .expect("principal should be extracted");

        assert_eq!(principal, "luke@monkeymachine");
    }

    #[test]
    fn email_matching_does_not_accept_the_short_form() {
        let mut extractor = SubjectX500PrincipalExtractor::new();
        extractor.set_extract_principal_name_from_email(true);

        let error = extractor
            .extract_principal(&X509Certificate::new(
                "EMAILADDRESS=luke@monkeymachine,O=Example Corp,C=US",
            ))
            .expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }

    #[test]
    fn fails_when_no_attribute_matches() {
        let error = extract("O=Example Corp,C=US").expect_err("extraction should fail");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }
}
