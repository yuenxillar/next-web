//! The attestation object of a registration.
//!
//! A registration answers with a CBOR map of three entries: the format of the
//! attestation statement, the statement itself, and the authenticator data of
//! the ceremony. Only the `none` format is believed here — the formats that
//! carry a certificate chain are refused by name, because a relying party that
//! cannot read a statement must say so rather than accept it.

use std::{collections::BTreeSet, error::Error, fmt};

use coset::cbor::value::Value;

use super::{
    authenticator_data::{AttestedCredentialData, AuthenticatorData, AuthenticatorDataError},
    cose::{CoseError, CosePublicKey},
};

/// The entry of an attestation object that names the format of its statement.
const FORMAT: &str = "fmt";

/// The entry of an attestation object that carries the statement itself.
const STATEMENT: &str = "attStmt";

/// The entry of an attestation object that carries the authenticator data.
const AUTHENTICATOR_DATA: &str = "authData";

/// The only attestation format this port can verify.
pub const NONE: &str = "none";

/// The attestation object of a registration ceremony.
///
/// The object is the CBOR map a registration answers with, decoded into the
/// three things the relying party needs: which statement was carried, whether
/// it is empty, and the authenticator data that the credential is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationObject {
    format: String,
    authenticator_data: AuthenticatorData,
    statement_is_empty: bool,
}

impl AttestationObject {
    /// Parses the attestation object a browser submits.
    pub fn parse(raw: &[u8]) -> Result<Self, AttestationError> {
        let value = Self::decode(raw)?;
        let Value::Map(entries) = value else {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                "the attestation object is not a CBOR map",
            ));
        };

        let mut seen = BTreeSet::new();
        let mut format = None;
        let mut authenticator_data = None;
        let mut statement_is_empty = None;

        for (key, value) in entries {
            let Some(key) = value_key(&key) else {
                return Err(AttestationError::new(
                    AttestationErrorKind::Malformed,
                    "the attestation object holds an entry whose name is not a text string",
                ));
            };
            if !seen.insert(key.to_string()) {
                return Err(AttestationError::new(
                    AttestationErrorKind::Malformed,
                    format!("the attestation object holds the entry {key} twice"),
                ));
            }

            match key {
                FORMAT => {
                    format = Some(text_of(value, FORMAT)?);
                }
                STATEMENT => {
                    let Value::Map(statement) = value else {
                        return Err(AttestationError::new(
                            AttestationErrorKind::Malformed,
                            "the attestation statement is not a CBOR map",
                        ));
                    };
                    statement_is_empty = Some(statement.is_empty());
                }
                AUTHENTICATOR_DATA => {
                    let Value::Bytes(bytes) = value else {
                        return Err(AttestationError::new(
                            AttestationErrorKind::Malformed,
                            "the authenticator data of the attestation object is not a byte string",
                        ));
                    };
                    authenticator_data = Some(AuthenticatorData::parse(&bytes)?);
                }
                // An entry this port does not know is not a reason to refuse an
                // attestation: the object is read by the entries that matter.
                _ => {}
            }
        }

        let Some(format) = format else {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                "the attestation object names no format",
            ));
        };
        let Some(authenticator_data) = authenticator_data else {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                "the attestation object carries no authenticator data",
            ));
        };
        let Some(statement_is_empty) = statement_is_empty else {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                "the attestation object carries no attestation statement",
            ));
        };

        Ok(Self {
            format,
            authenticator_data,
            statement_is_empty,
        })
    }

    /// Decodes the CBOR of the object, and refuses bytes that follow it.
    fn decode(raw: &[u8]) -> Result<Value, AttestationError> {
        let mut reader = raw;
        let value: Value = ciborium::de::from_reader(&mut reader).map_err(|error| {
            AttestationError::new(
                AttestationErrorKind::Malformed,
                format!("the attestation object is not CBOR: {error}"),
            )
        })?;
        if !reader.is_empty() {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                format!(
                    "{} bytes follow the attestation object, which is a single CBOR map",
                    reader.len()
                ),
            ));
        }

        Ok(value)
    }

    /// Returns the format of the attestation statement.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Returns the authenticator data of the ceremony.
    pub fn authenticator_data(&self) -> &AuthenticatorData {
        &self.authenticator_data
    }

    /// Returns whether the attestation statement carried no entry.
    pub fn statement_is_empty(&self) -> bool {
        self.statement_is_empty
    }

    /// Checks that the statement is one this port can verify.
    ///
    /// The `none` format says that the authenticator attested nothing, and it
    /// carries an empty statement. Every other format carries a signature over
    /// a certificate chain, which this port has no verifier for — so it is
    /// refused by name instead of being believed unverified.
    pub fn validate(&self) -> Result<(), AttestationError> {
        if self.format != NONE {
            return Err(AttestationError::new(
                AttestationErrorKind::UnsupportedFormat,
                format!(
                    "the attestation statement is of the format {}, which this relying party \
                     cannot verify; only {NONE} is accepted",
                    self.format
                ),
            ));
        }
        if !self.statement_is_empty {
            return Err(AttestationError::new(
                AttestationErrorKind::NonEmptyStatement,
                format!("the attestation statement of the {NONE} format is empty, and this one is not"),
            ));
        }

        Ok(())
    }

    /// Reads the credential a registration attests.
    ///
    /// The credential id is read from the fixed layout that precedes the key,
    /// and the key is the COSE key that follows it. What comes after the key is
    /// the extensions of the authenticator data, which are a CBOR map: the key
    /// is only separable from them because the decoder says where it ends.
    pub fn attested_credential(&self) -> Result<AttestedCredential, AttestationError> {
        if !self.authenticator_data.has_attested_credential_data() {
            return Err(AttestationError::new(
                AttestationErrorKind::MissingCredential,
                "the flags of the authenticator data of a registration say that the attested \
                 credential data follows, and these do not",
            ));
        }

        let data = AttestedCredentialData::parse(self.authenticator_data.remainder())?;
        let (public_key, consumed) = CosePublicKey::parse_prefix(data.credential_public_key())?;
        let extensions = &data.credential_public_key()[consumed..];
        Self::check_extensions(extensions)?;

        let aaguid: [u8; 16] = data.aaguid().try_into().map_err(|_| {
            AttestationError::new(
                AttestationErrorKind::Malformed,
                "the authenticator data holds an AAGUID that is not sixteen bytes",
            )
        })?;

        Ok(AttestedCredential {
            aaguid,
            credential_id: data.credential_id().to_vec(),
            cose_public_key: data.credential_public_key()[..consumed].to_vec(),
            public_key,
        })
    }

    /// Checks that what follows the public key is the extensions of the data.
    fn check_extensions(extensions: &[u8]) -> Result<(), AttestationError> {
        if extensions.is_empty() {
            return Ok(());
        }

        let mut reader = extensions;
        let value: Value = ciborium::de::from_reader(&mut reader).map_err(|error| {
            AttestationError::new(
                AttestationErrorKind::Malformed,
                format!("the extensions of the authenticator data are not CBOR: {error}"),
            )
        })?;
        if !reader.is_empty() || value.as_map().is_none() {
            return Err(AttestationError::new(
                AttestationErrorKind::Malformed,
                "the extensions of the authenticator data are a single CBOR map, and these are not",
            ));
        }

        Ok(())
    }
}

/// Returns the name of a CBOR map key, when it is a text string.
fn value_key(key: &Value) -> Option<&str> {
    key.as_text()
}

/// Returns the text of an entry of the attestation object.
fn text_of(value: Value, entry: &str) -> Result<String, AttestationError> {
    match value {
        Value::Text(text) => Ok(text),
        _ => Err(AttestationError::new(
            AttestationErrorKind::Malformed,
            format!("the entry {entry} of the attestation object is not a text string"),
        )),
    }
}

/// The credential a registration attested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedCredential {
    aaguid: [u8; 16],
    credential_id: Vec<u8>,
    cose_public_key: Vec<u8>,
    public_key: CosePublicKey,
}

impl AttestedCredential {
    /// Returns the identifier of the authenticator model.
    pub fn aaguid(&self) -> &[u8; 16] {
        &self.aaguid
    }

    /// Returns the identifier the authenticator chose for the credential.
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// Returns the public key exactly as the authenticator encoded it, which is
    /// what a later authentication is verified against.
    pub fn cose_public_key(&self) -> &[u8] {
        &self.cose_public_key
    }

    /// Returns the decoded public key.
    pub fn public_key(&self) -> &CosePublicKey {
        &self.public_key
    }
}

/// What went wrong while an attestation object was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationErrorKind {
    /// The bytes are not the attestation object they claim to be.
    Malformed,
    /// The statement is of a format this port cannot verify.
    UnsupportedFormat,
    /// The statement of the `none` format carries an entry.
    NonEmptyStatement,
    /// A registration carried no attested credential.
    MissingCredential,
}

/// The error of reading or checking an attestation object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationError {
    kind: AttestationErrorKind,
    message: String,
}

impl AttestationError {
    /// Creates the error of the given kind with the given message.
    pub fn new(kind: AttestationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns what went wrong.
    pub fn kind(&self) -> AttestationErrorKind {
        self.kind
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AttestationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AttestationError {}

impl From<AuthenticatorDataError> for AttestationError {
    fn from(error: AuthenticatorDataError) -> Self {
        Self::new(
            AttestationErrorKind::Malformed,
            format!("the authenticator data of the attestation object is not valid: {error}"),
        )
    }
}

impl From<CoseError> for AttestationError {
    fn from(error: CoseError) -> Self {
        Self::new(AttestationErrorKind::Malformed, error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webauthn::test_support::{
        cbor_bytes, cbor_integer, cbor_map, cbor_text, es256_cose_key,
    };

    /// The flag that says the user took part in the ceremony.
    const USER_PRESENT: u8 = 0x01;

    /// The flag that says the attested credential data follows the header.
    const ATTESTED_CREDENTIAL_DATA: u8 = 0x40;

    /// Returns authenticator data with the given flags and, when one is given,
    /// the attested credential of a registration.
    fn authenticator_data(flags: u8, credential: Option<(&[u8], &[u8])>) -> Vec<u8> {
        let mut raw = Vec::new();
        raw.extend_from_slice(&[0x5a; 32]);
        raw.push(flags);
        raw.extend_from_slice(&7u32.to_be_bytes());
        if let Some((credential_id, public_key)) = credential {
            raw.extend_from_slice(&[0x11; 16]);
            raw.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
            raw.extend_from_slice(credential_id);
            raw.extend_from_slice(public_key);
        }

        raw
    }

    /// Returns an attestation object of the given format and statement.
    fn attestation_object(
        format: &str,
        statement: &[(Vec<u8>, Vec<u8>)],
        authenticator_data: &[u8],
    ) -> Vec<u8> {
        cbor_map(&[
            (cbor_text("fmt"), cbor_text(format)),
            (cbor_text("attStmt"), cbor_map(statement)),
            (cbor_text("authData"), cbor_bytes(authenticator_data)),
        ])
    }

    /// Returns the attestation object of a registration the tests accept.
    fn none_attestation(authenticator_data: &[u8]) -> Vec<u8> {
        attestation_object("none", &[], authenticator_data)
    }

    #[test]
    fn an_attestation_object_of_the_none_format_is_read() {
        let raw = none_attestation(&authenticator_data(
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            Some((b"the-credential", &es256_cose_key())),
        ));

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");

        assert_eq!(attestation.format(), "none");
        assert_eq!(attestation.authenticator_data().sign_count(), 7);
    }

    #[test]
    fn the_none_format_carries_no_statement() {
        let raw = none_attestation(&authenticator_data(USER_PRESENT, None));

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");

        attestation
            .validate()
            .expect("the none format is the one this relying party believes");
    }

    #[test]
    fn the_credential_the_attestation_carries_is_read() {
        let cose_key = es256_cose_key();
        let raw = none_attestation(&authenticator_data(
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            Some((b"the-credential", &cose_key)),
        ));

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let credential = attestation
            .attested_credential()
            .expect("a registration attests a credential");

        assert_eq!(credential.aaguid(), &[0x11; 16]);
        assert_eq!(credential.credential_id(), b"the-credential");
        assert_eq!(
            credential.cose_public_key(),
            cose_key.as_slice(),
            "the key is kept as the authenticator wrote it"
        );
        assert_eq!(credential.public_key().algorithm(), -7);
    }

    #[test]
    fn a_registration_without_an_attested_credential_is_refused() {
        let raw = none_attestation(&authenticator_data(USER_PRESENT, None));

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let error = attestation
            .attested_credential()
            .expect_err("a registration has to attest a credential");

        assert_eq!(error.kind(), AttestationErrorKind::MissingCredential);
    }

    #[test]
    fn the_format_of_another_implementation_is_refused_by_name() {
        let raw = attestation_object(
            "packed",
            &[],
            &authenticator_data(USER_PRESENT | ATTESTED_CREDENTIAL_DATA, None),
        );

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let error = attestation
            .validate()
            .expect_err("no certificate chain can be verified by this port");

        assert_eq!(error.kind(), AttestationErrorKind::UnsupportedFormat);
        assert!(
            error.message().contains("packed"),
            "the error names the format it cannot read, was: {error}"
        );
    }

    #[test]
    fn a_statement_the_none_format_must_not_carry_is_refused() {
        let raw = attestation_object(
            "none",
            &[(cbor_integer(1), cbor_integer(-7))],
            &authenticator_data(USER_PRESENT, None),
        );

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let error = attestation
            .validate()
            .expect_err("the none format carries an empty statement");

        assert_eq!(error.kind(), AttestationErrorKind::NonEmptyStatement);
    }

    #[test]
    fn the_extensions_that_follow_the_key_are_kept_apart() {
        let mut raw_authenticator_data = authenticator_data(
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            Some((b"the-credential", &es256_cose_key())),
        );
        raw_authenticator_data.extend_from_slice(&cbor_map(&[(
            cbor_text("credProtect"),
            cbor_integer(2),
        )]));
        let raw = none_attestation(&raw_authenticator_data);

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let credential = attestation
            .attested_credential()
            .expect("a registration attests a credential");

        assert_eq!(
            credential.cose_public_key(),
            es256_cose_key().as_slice(),
            "the extensions are not read as part of the key"
        );
    }

    #[test]
    fn extensions_that_are_not_a_map_are_refused() {
        let mut raw_authenticator_data = authenticator_data(
            USER_PRESENT | ATTESTED_CREDENTIAL_DATA,
            Some((b"the-credential", &es256_cose_key())),
        );
        raw_authenticator_data.extend_from_slice(&cbor_integer(5));
        let raw = none_attestation(&raw_authenticator_data);

        let attestation = AttestationObject::parse(&raw).expect("the attestation object parses");
        let error = attestation
            .attested_credential()
            .expect_err("the extensions are a CBOR map");

        assert_eq!(error.kind(), AttestationErrorKind::Malformed);
    }

    #[test]
    fn an_attestation_object_without_authenticator_data_is_refused() {
        let raw = cbor_map(&[
            (cbor_text("fmt"), cbor_text("none")),
            (cbor_text("attStmt"), cbor_map(&[])),
        ]);

        let error = AttestationObject::parse(&raw).expect_err("the authenticator data is missing");

        assert_eq!(error.kind(), AttestationErrorKind::Malformed);
    }

    #[test]
    fn an_attestation_object_that_names_its_format_twice_is_refused() {
        let raw = cbor_map(&[
            (cbor_text("fmt"), cbor_text("none")),
            (cbor_text("fmt"), cbor_text("packed")),
            (cbor_text("attStmt"), cbor_map(&[])),
            (
                cbor_text("authData"),
                cbor_bytes(&authenticator_data(USER_PRESENT, None)),
            ),
        ]);

        let error = AttestationObject::parse(&raw).expect_err("a map has one value per key");

        assert_eq!(error.kind(), AttestationErrorKind::Malformed);
        assert!(
            error.message().contains("fmt"),
            "the error names the entry that was repeated, was: {error}"
        );
    }

    #[test]
    fn an_attestation_object_that_is_not_a_map_is_refused() {
        let error = AttestationObject::parse(b"not a CBOR map at all").expect_err("this is not CBOR");

        assert_eq!(error.kind(), AttestationErrorKind::Malformed);
    }

    #[test]
    fn an_attestation_object_whose_authenticator_data_is_short_is_refused() {
        let raw = none_attestation(&[0x5a; 16]);

        let error = AttestationObject::parse(&raw)
            .expect_err("the header of the authenticator data is 37 bytes");

        assert_eq!(error.kind(), AttestationErrorKind::Malformed);
    }
}
