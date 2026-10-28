//! The subject distinguished name of a certificate.
//!
//! The certificate a client presents arrives as its DER encoding, and the
//! subject it names is what the X.509 filter reads. This module turns the one
//! into the other, in the spelling RFC 4514 defines: the attributes from the most
//! specific to the least specific, and every character that would change the
//! meaning of the name escaped.

use simple_asn1::{ASN1Block, ASN1Class, BigUint, OID};

use crate::core::{AuthenticationError, AuthenticationErrorKind};

/// Returns the subject distinguished name of a DER-encoded certificate.
///
/// # Errors
///
/// Returns [`AuthenticationErrorKind::BadCredentials`] when the bytes are not a
/// certificate, or when its subject cannot be read.
pub fn subject_dn(encoded: &[u8]) -> Result<String, AuthenticationError> {
    let blocks = simple_asn1::from_der(encoded).map_err(|error| {
        AuthenticationError::with_kind(
            format!("the certificate is not DER: {error}"),
            AuthenticationErrorKind::BadCredentials,
        )
    })?;

    let certificate = match blocks.as_slice() {
        [ASN1Block::Sequence(_, items)] => items,
        _ => return Err(not_a_certificate("it is not a single sequence")),
    };
    let tbs = match certificate.first() {
        Some(ASN1Block::Sequence(_, items)) => items,
        _ => return Err(not_a_certificate("it holds no sequence of certificate fields")),
    };

    // The subject follows the optional version, the serial number, the signature
    // algorithm, the issuer and the validity.
    let subject = match tbs.first() {
        Some(ASN1Block::Explicit(ASN1Class::ContextSpecific, _, tag, _)) if *tag == BigUint::from(0u8) => {
            tbs.get(5)
        }
        _ => tbs.get(4),
    };
    let Some(ASN1Block::Sequence(_, names)) = subject else {
        return Err(not_a_certificate("it names no subject"));
    };

    let mut attributes = Vec::new();
    for name in names {
        let ASN1Block::Set(_, values) = name else {
            return Err(not_a_certificate("a name of the subject is not a set"));
        };
        for value in values {
            let ASN1Block::Sequence(_, pair) = value else {
                return Err(not_a_certificate(
                    "an attribute of the subject is not a sequence",
                ));
            };
            let (Some(ASN1Block::ObjectIdentifier(_, oid)), Some(block)) =
                (pair.first(), pair.get(1))
            else {
                return Err(not_a_certificate(
                    "an attribute of the subject has no type or no value",
                ));
            };
            attributes.push(format!(
                "{}={}",
                attribute_type(oid),
                escape(&value_of(block)?)
            ));
        }
    }

    // RFC 4514 spells a name from the most specific attribute to the least
    // specific one, which is the reverse of the order the encoding uses.
    attributes.reverse();

    Ok(attributes.join(", "))
}

/// Returns the error of bytes that are not a certificate.
fn not_a_certificate(reason: &str) -> AuthenticationError {
    AuthenticationError::with_kind(
        format!("the certificate cannot be read: {reason}"),
        AuthenticationErrorKind::BadCredentials,
    )
}

/// Returns the text of an attribute value.
fn value_of(block: &ASN1Block) -> Result<String, AuthenticationError> {
    match block {
        ASN1Block::UTF8String(_, value)
        | ASN1Block::PrintableString(_, value)
        | ASN1Block::TeletexString(_, value)
        | ASN1Block::IA5String(_, value)
        | ASN1Block::UniversalString(_, value)
        | ASN1Block::BMPString(_, value) => Ok(value.clone()),
        other => Err(not_a_certificate(&format!(
            "the value {:?} of an attribute is not a string",
            other
        ))),
    }
}

/// Returns the type of an attribute, named the way a distinguished name spells
/// it.
fn attribute_type(oid: &OID) -> String {
    let components = oid.as_vec::<u64>().unwrap_or_default();
    let name = match components.as_slice() {
        [2, 5, 4, 3] => "CN",
        [2, 5, 4, 4] => "SN",
        [2, 5, 4, 5] => "serialNumber",
        [2, 5, 4, 6] => "C",
        [2, 5, 4, 7] => "L",
        [2, 5, 4, 8] => "ST",
        [2, 5, 4, 9] => "street",
        [2, 5, 4, 10] => "O",
        [2, 5, 4, 11] => "OU",
        [2, 5, 4, 12] => "title",
        [2, 5, 4, 42] => "givenName",
        [1, 2, 840, 113549, 1, 9, 1] => "emailAddress",
        [0, 9, 2342, 19200300, 100, 1, 25] => "DC",
        [0, 9, 2342, 19200300, 100, 1, 1] => "UID",
        other => return other.iter().map(u64::to_string).collect::<Vec<_>>().join("."),
    };

    name.to_string()
}

/// Returns a value in the escaping RFC 4514 defines.
///
/// Two attributes are joined by a comma and a type is separated from its value by
/// an equals sign, so a value that carries one of those characters has to escape
/// it: otherwise a certificate could name itself an attribute it was not given.
fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for (index, character) in value.chars().enumerate() {
        let last = index + 1 == value.chars().count();
        match character {
            ',' | '+' | '"' | '\\' | '<' | '>' | ';' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '#' if index == 0 => {
                escaped.push('\\');
                escaped.push(character);
            }
            ' ' if index == 0 || last => {
                escaped.push('\\');
                escaped.push(character);
            }
            other => escaped.push(other),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    /// Returns the DER of the client certificate the TLS fixtures carry.
    fn client_certificate() -> Vec<u8> {
        let pem = include_str!("../../../../../../tests/fixtures/tls/client-cert.pem");
        let body = pem
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<String>();

        base64::engine::general_purpose::STANDARD
            .decode(body)
            .expect("the fixture is base64")
    }

    #[test]
    fn the_subject_of_a_certificate_is_read() {
        let subject = subject_dn(&client_certificate()).expect("the subject is read");

        assert_eq!(
            subject, "O=Example, CN=Alice",
            "the encoding names the common name first, so the string names it last"
        );
    }

    #[test]
    fn bytes_that_are_not_a_certificate_are_refused() {
        let error = subject_dn(b"not a certificate").expect_err("the bytes are not DER");

        assert_eq!(error.kind(), AuthenticationErrorKind::BadCredentials);
    }

    #[test]
    fn a_value_that_carries_a_separator_is_escaped() {
        assert_eq!(escape("Alice, O=Example"), "Alice\\, O=Example");
        assert_eq!(escape("#Alice"), "\\#Alice");
        assert_eq!(escape(" Alice "), "\\ Alice\\ ");
        assert_eq!(escape("plain"), "plain");
    }
}
