//! The authenticator data of a ceremony.
//!
//! The first thirty-seven bytes are fixed: a hash of the relying party the
//! credential belongs to, the flags of the ceremony, and the signature counter of
//! the authenticator. What follows depends on the flags — the attested credential
//! data of a registration and the extensions — and both are encoded with CBOR, so
//! this module keeps them as the bytes they arrived as and leaves their decoding
//! to the phase that has a CBOR decoder.

use std::{error::Error, fmt};

/// The authenticator data of a ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatorData {
    rp_id_hash: [u8; 32],
    flags: u8,
    sign_count: u32,
    remainder: Vec<u8>,
}

impl AuthenticatorData {
    /// The number of bytes the header of every ceremony occupies.
    pub const HEADER_LENGTH: usize = 37;

    /// The flag that says the user took part in the ceremony.
    pub const USER_PRESENT: u8 = 0x01;

    /// The flag that says the user was verified, by a pin or a biometry.
    pub const USER_VERIFIED: u8 = 0x04;

    /// The flag that says the attested credential data follows the header.
    pub const ATTESTED_CREDENTIAL_DATA: u8 = 0x40;

    /// The flag that says the extensions follow.
    pub const EXTENSION_DATA: u8 = 0x80;

    /// Parses the authenticator data of a ceremony.
    pub fn parse(raw: &[u8]) -> Result<Self, AuthenticatorDataError> {
        if raw.len() < Self::HEADER_LENGTH {
            return Err(AuthenticatorDataError::new(format!(
                "the authenticator data is {} bytes, shorter than the {} of its header",
                raw.len(),
                Self::HEADER_LENGTH
            )));
        }

        let rp_id_hash: [u8; 32] = raw[..32]
            .try_into()
            .expect("the slice is thirty-two bytes long");
        let sign_count = u32::from_be_bytes(
            raw[33..Self::HEADER_LENGTH]
                .try_into()
                .expect("the slice is four bytes long"),
        );

        Ok(Self {
            rp_id_hash,
            flags: raw[32],
            sign_count,
            remainder: raw[Self::HEADER_LENGTH..].to_vec(),
        })
    }

    /// Returns the hash of the relying party the credential belongs to.
    pub fn rp_id_hash(&self) -> &[u8] {
        &self.rp_id_hash
    }

    /// Returns the flags of the ceremony.
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// Returns how many times the authenticator produced an assertion.
    ///
    /// The counter is what makes a cloned authenticator visible: it only grows,
    /// so a value that does not grow is the signal.
    pub fn sign_count(&self) -> u32 {
        self.sign_count
    }

    /// Returns whether the user took part in the ceremony.
    pub fn user_present(&self) -> bool {
        self.flags & Self::USER_PRESENT != 0
    }

    /// Returns whether the user was verified by the authenticator.
    pub fn user_verified(&self) -> bool {
        self.flags & Self::USER_VERIFIED != 0
    }

    /// Returns whether the attested credential data follows the header.
    pub fn has_attested_credential_data(&self) -> bool {
        self.flags & Self::ATTESTED_CREDENTIAL_DATA != 0
    }

    /// Returns the bytes that follow the header, whatever they are.
    ///
    /// The lengths of the attested credential data and of the extensions are only
    /// known once they are decoded, so they are kept together here rather than
    /// split into fields that could only be guessed.
    pub fn remainder(&self) -> &[u8] {
        &self.remainder
    }

    /// Checks the data against what the relying party expects of the ceremony.
    pub fn validate(
        &self,
        expected_rp_id_hash: &[u8],
        require_user_verification: bool,
    ) -> Result<(), AuthenticatorDataError> {
        if self.rp_id_hash != expected_rp_id_hash {
            return Err(AuthenticatorDataError::new(
                "the authenticator data belongs to another relying party",
            ));
        }
        if !self.user_present() {
            return Err(AuthenticatorDataError::new(
                "the user did not take part in the ceremony",
            ));
        }
        if require_user_verification && !self.user_verified() {
            return Err(AuthenticatorDataError::new(
                "the ceremony requires a verified user and the authenticator did not verify one",
            ));
        }

        Ok(())
    }
}

/// The error of reading or checking the authenticator data of a ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatorDataError {
    message: String,
}

impl AuthenticatorDataError {
    /// Creates the error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AuthenticatorDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AuthenticatorDataError {}

/// The credential a registration ceremony attested.
///
/// The layout is fixed up to the public key: sixteen bytes of authenticator
/// identifier, two bytes of length, then the credential id. Only the public key
/// that follows is CBOR, so the id can be read without a decoder — which is what
/// makes the registration storable at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestedCredentialData {
    aaguid: [u8; 16],
    credential_id: Vec<u8>,
    credential_public_key: Vec<u8>,
}

impl AttestedCredentialData {
    /// The number of bytes that precede the length of the credential id.
    pub const PREFIX_LENGTH: usize = 18;

    /// Parses the authenticated credential data of a registration.
    pub fn parse(raw: &[u8]) -> Result<Self, AuthenticatorDataError> {
        if raw.len() < Self::PREFIX_LENGTH {
            return Err(AuthenticatorDataError::new(format!(
                "the attested credential data is {} bytes, shorter than the {} of its prefix",
                raw.len(),
                Self::PREFIX_LENGTH
            )));
        }

        let aaguid: [u8; 16] = raw[..16]
            .try_into()
            .expect("the slice is sixteen bytes long");
        let credential_id_length = u16::from_be_bytes([raw[16], raw[17]]) as usize;
        let credential_id_end = Self::PREFIX_LENGTH + credential_id_length;

        if raw.len() < credential_id_end {
            return Err(AuthenticatorDataError::new(format!(
                "the attested credential data claims a credential id of {credential_id_length} bytes, \
                 which does not fit in the {} bytes that follow",
                raw.len() - Self::PREFIX_LENGTH
            )));
        }

        Ok(Self {
            aaguid,
            credential_id: raw[Self::PREFIX_LENGTH..credential_id_end].to_vec(),
            credential_public_key: raw[credential_id_end..].to_vec(),
        })
    }

    /// Returns the identifier of the authenticator model, which a relying party
    /// may use to recognise one.
    pub fn aaguid(&self) -> &[u8] {
        &self.aaguid
    }

    /// Returns the identifier the authenticator chose for the credential.
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }

    /// Returns the public key of the credential, still encoded as a COSE key.
    pub fn credential_public_key(&self) -> &[u8] {
        &self.credential_public_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns authenticator data with the given flags and counter.
    fn authenticator_data(flags: u8, sign_count: u32) -> Vec<u8> {
        let mut raw = Vec::with_capacity(AuthenticatorData::HEADER_LENGTH);
        raw.extend_from_slice(&[0x5a; 32]);
        raw.push(flags);
        raw.extend_from_slice(&sign_count.to_be_bytes());
        raw.extend_from_slice(b"the attested credential data");

        raw
    }

    #[test]
    fn the_header_of_a_ceremony_is_parsed() {
        let data = AuthenticatorData::parse(&authenticator_data(
            AuthenticatorData::USER_PRESENT | AuthenticatorData::USER_VERIFIED,
            7,
        ))
        .expect("the authenticator data parses");

        assert_eq!(data.rp_id_hash(), &[0x5a; 32]);
        assert_eq!(data.sign_count(), 7);
        assert!(data.user_present());
        assert!(data.user_verified());
        assert_eq!(data.remainder(), b"the attested credential data");
    }

    #[test]
    fn data_that_is_shorter_than_the_header_is_refused() {
        assert!(AuthenticatorData::parse(&[0u8; 36]).is_err());
    }

    #[test]
    fn data_of_another_relying_party_is_refused() {
        let data = AuthenticatorData::parse(&authenticator_data(AuthenticatorData::USER_PRESENT, 1))
            .expect("the authenticator data parses");

        let error = data
            .validate(&[0x5b; 32], false)
            .expect_err("the data belongs to another relying party");

        assert!(error.message().contains("another relying party"), "was: {error}");
    }

    #[test]
    fn data_of_a_ceremony_without_the_user_is_refused() {
        let data = AuthenticatorData::parse(&authenticator_data(0, 1))
            .expect("the authenticator data parses");

        let error = data
            .validate(&[0x5a; 32], false)
            .expect_err("the user did not take part in the ceremony");

        assert!(error.message().contains("did not take part"), "was: {error}");
    }

    #[test]
    fn data_of_an_unverified_user_is_refused_when_verification_is_required() {
        let data = AuthenticatorData::parse(&authenticator_data(
            AuthenticatorData::USER_PRESENT,
            1,
        ))
        .expect("the authenticator data parses");

        data.validate(&[0x5a; 32], false)
            .expect("the ceremony does not require a verified user");

        let error = data
            .validate(&[0x5a; 32], true)
            .expect_err("the ceremony requires a verified user");

        assert!(error.message().contains("verified user"), "was: {error}");
    }

    #[test]
    fn the_flag_of_the_attested_credential_data_is_reported() {
        let data = AuthenticatorData::parse(&authenticator_data(
            AuthenticatorData::USER_PRESENT | AuthenticatorData::ATTESTED_CREDENTIAL_DATA,
            1,
        ))
        .expect("the authenticator data parses");

        assert!(data.has_attested_credential_data());
    }

    #[test]
    fn the_credential_of_a_registration_is_read_without_a_cbor_decoder() {
        let mut raw = vec![0x11; 16];
        raw.extend_from_slice(&4u16.to_be_bytes());
        raw.extend_from_slice(b"cred");
        raw.extend_from_slice(b"the cose public key");

        let credential =
            AttestedCredentialData::parse(&raw).expect("the attested credential data parses");

        assert_eq!(credential.aaguid(), &[0x11; 16]);
        assert_eq!(credential.credential_id(), b"cred");
        assert_eq!(credential.credential_public_key(), b"the cose public key");
    }

    #[test]
    fn a_credential_id_that_does_not_fit_is_refused() {
        let mut raw = vec![0x11; 16];
        raw.extend_from_slice(&9u16.to_be_bytes());
        raw.extend_from_slice(b"cred");

        assert!(AttestedCredentialData::parse(&raw).is_err());
    }

    #[test]
    fn attested_credential_data_shorter_than_its_prefix_is_refused() {
        assert!(AttestedCredentialData::parse(&[0u8; 17]).is_err());
    }
}
