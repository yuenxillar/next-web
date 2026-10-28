//! The public key of a credential, and the signature over a ceremony.
//!
//! A credential of a Web Authentication ceremony travels as a COSE key: a
//! small CBOR map whose integer labels name the key type, the signature
//! algorithm, the curve and the coordinates or the RSA modulus. The
//! authenticator then signs `authenticatorData || SHA-256(clientDataJSON)`
//! with the private half of that key.
//!
//! The two encodings are the reason this phase exists: the key is only a
//! key once it is decoded, and the signature is only a signature once the
//! algorithm it names is applied to the two halves of the ceremony.

use std::{error::Error, fmt};

use coset::{
    cbor::value::Value, iana, AsCborValue, CoseKey, KeyType, Label,
    RegisteredLabelWithPrivate,
};

/// The COSE identifier of ECDSA with SHA-256 over P-256.
pub const ES256: i64 = -7;

/// The COSE identifier of RSASSA-PKCS1-v1_5 with SHA-256.
pub const RS256: i64 = -257;

/// The COSE identifier of the only curve this port can use.
const P_256: i64 = 1;

/// The material of a public key, as the COSE key carried it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyMaterial {
    /// An elliptic curve point of the given curve.
    Ec2 {
        curve: i64,
        x: Vec<u8>,
        y: Vec<u8>,
    },
    /// An RSA modulus and public exponent.
    Rsa {
        modulus: Vec<u8>,
        exponent: Vec<u8>,
    },
}

/// The public key of a credential, decoded from its COSE form.
///
/// Only the two algorithms every authenticator has to support are accepted:
/// ES256 and RS256. The others are refused by number instead of being
/// silently ignored, because a key whose algorithm the relying party cannot
/// apply is a credential it could never verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CosePublicKey {
    algorithm: i64,
    material: KeyMaterial,
}

impl CosePublicKey {
    /// Decodes a COSE key that occupies the whole slice.
    pub fn parse(raw: &[u8]) -> Result<Self, CoseError> {
        let (key, consumed) = Self::parse_prefix(raw)?;
        if consumed != raw.len() {
            return Err(CoseError::new(
                CoseErrorKind::Malformed,
                format!(
                    "the COSE key occupies {consumed} bytes and {} follow it",
                    raw.len() - consumed
                ),
            ));
        }

        Ok(key)
    }

    /// Decodes a COSE key, and reports how many bytes it occupies.
    ///
    /// The bytes that follow the key are the extensions of the authenticator
    /// data, which is why the length matters: only a decoder knows where the
    /// key ends.
    pub fn parse_prefix(raw: &[u8]) -> Result<(Self, usize), CoseError> {
        let mut reader = raw;
        let value: Value = ciborium::de::from_reader(&mut reader).map_err(|error| {
            CoseError::new(
                CoseErrorKind::Malformed,
                format!("the COSE key is not CBOR: {error}"),
            )
        })?;
        let consumed = raw.len() - reader.len();
        let key = CoseKey::from_cbor_value(value).map_err(|error| {
            CoseError::new(
                CoseErrorKind::Malformed,
                format!("the COSE key does not hold the fields of a COSE key: {error}"),
            )
        })?;

        Self::from_cose_key(key).map(|key| (key, consumed))
    }

    /// Reads the algorithm and the material out of a decoded COSE key.
    fn from_cose_key(key: CoseKey) -> Result<Self, CoseError> {
        let algorithm = Self::algorithm_of(&key)?;

        let material = match algorithm {
            ES256 => Self::ec2_material(&key)?,
            RS256 => Self::rsa_material(&key)?,
            _ => {
                return Err(CoseError::new(
                    CoseErrorKind::UnsupportedAlgorithm,
                    format!(
                        "the COSE key names algorithm {algorithm}, which this port does not \
                         implement; ES256 ({ES256}) and RS256 ({RS256}) are the two it does"
                    ),
                ))
            }
        };

        Ok(Self {
            algorithm,
            material,
        })
    }

    /// Returns the algorithm of the key, or refuses a key that names none.
    fn algorithm_of(key: &CoseKey) -> Result<i64, CoseError> {
        let Some(algorithm) = &key.alg else {
            return Err(CoseError::new(
                CoseErrorKind::UnsupportedAlgorithm,
                "the COSE key names no algorithm, so the relying party cannot tell how to \
                 verify a signature with it",
            ));
        };

        match algorithm {
            RegisteredLabelWithPrivate::Assigned(_) | RegisteredLabelWithPrivate::PrivateUse(_) => {
                Self::label_number(algorithm).ok_or_else(|| {
                    CoseError::new(
                        CoseErrorKind::UnsupportedAlgorithm,
                        "the COSE key names an algorithm this port cannot read",
                    )
                })
            }
            RegisteredLabelWithPrivate::Text(text) => Err(CoseError::new(
                CoseErrorKind::UnsupportedAlgorithm,
                format!("the COSE key names the algorithm {text}, which is not a number"),
            )),
        }
    }

    /// Returns the number a label of the CBOR form of the key carries.
    fn label_number(label: &RegisteredLabelWithPrivate<iana::Algorithm>) -> Option<i64> {
        label
            .clone()
            .to_cbor_value()
            .ok()
            .and_then(|value| value.as_integer())
            .and_then(|value| i64::try_from(value).ok())
    }

    /// Returns the point of an ES256 key.
    fn ec2_material(key: &CoseKey) -> Result<KeyMaterial, CoseError> {
        if key.kty != KeyType::Assigned(iana::KeyType::EC2) {
            return Err(CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!(
                    "the COSE key of ES256 is of key type {:?}, not EC2",
                    key.kty
                ),
            ));
        }

        let curve = Self::integer_parameter(key, iana::Ec2KeyParameter::Crv as i64)
            .ok_or_else(|| {
                CoseError::new(
                    CoseErrorKind::UnsupportedKey,
                    "the COSE key of ES256 names no curve",
                )
            })?;
        if curve != P_256 {
            return Err(CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!(
                    "the COSE key is on {}, and this port can only use P-256",
                    Self::curve_name(curve)
                ),
            ));
        }

        let x = Self::bytes_parameter(key, iana::Ec2KeyParameter::X as i64)?;
        let y = Self::bytes_parameter(key, iana::Ec2KeyParameter::Y as i64)?;
        if x.len() != 32 || y.len() != 32 {
            return Err(CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!(
                    "the COSE key holds a point of {} and {} bytes, not the 32 and 32 of P-256",
                    x.len(),
                    y.len()
                ),
            ));
        }

        Ok(KeyMaterial::Ec2 { curve, x, y })
    }

    /// Returns the modulus and the exponent of an RS256 key.
    fn rsa_material(key: &CoseKey) -> Result<KeyMaterial, CoseError> {
        if key.kty != KeyType::Assigned(iana::KeyType::RSA) {
            return Err(CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!(
                    "the COSE key of RS256 is of key type {:?}, not RSA",
                    key.kty
                ),
            ));
        }

        Ok(KeyMaterial::Rsa {
            modulus: Self::bytes_parameter(key, iana::RsaKeyParameter::N as i64)?,
            exponent: Self::bytes_parameter(key, iana::RsaKeyParameter::E as i64)?,
        })
    }

    /// Returns an integer parameter of the key.
    fn integer_parameter(key: &CoseKey, label: i64) -> Option<i64> {
        key.params
            .iter()
            .find(|(candidate, _)| candidate == &Label::Int(label))
            .and_then(|(_, value)| value.as_integer())
            .and_then(|value| i64::try_from(value).ok())
    }

    /// Returns a byte string parameter of the key.
    fn bytes_parameter(key: &CoseKey, label: i64) -> Result<Vec<u8>, CoseError> {
        key.params
            .iter()
            .find(|(candidate, _)| candidate == &Label::Int(label))
            .and_then(|(_, value)| value.as_bytes().cloned())
            .ok_or_else(|| {
                CoseError::new(
                    CoseErrorKind::UnsupportedKey,
                    format!("the COSE key holds no byte string under the label {label}"),
                )
            })
    }

    /// Returns the name of a curve, for an error that a reader can act on.
    fn curve_name(curve: i64) -> String {
        match curve {
            1 => "P-256".to_string(),
            2 => "P-384".to_string(),
            3 => "P-521".to_string(),
            6 => "Ed25519".to_string(),
            other => format!("the curve {other}"),
        }
    }

    /// Returns the identifier of the signature algorithm of the key.
    pub fn algorithm(&self) -> i64 {
        self.algorithm
    }

    /// Returns the curve of an elliptic curve key, which an RSA key has none of.
    pub fn curve(&self) -> Option<i64> {
        match &self.material {
            KeyMaterial::Ec2 { curve, .. } => Some(*curve),
            KeyMaterial::Rsa { .. } => None,
        }
    }

    /// Verifies the signature of a ceremony.
    ///
    /// The bytes are those of `authenticatorData || SHA-256(clientDataJSON)`,
    /// and the hash of them is what the signature covers: the algorithm of the
    /// key names the hash as well as the arithmetic.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), CoseError> {
        match (&self.material, self.algorithm) {
            (KeyMaterial::Ec2 { x, y, .. }, ES256) => Self::verify_es256(x, y, message, signature),
            (KeyMaterial::Rsa { modulus, exponent }, RS256) => {
                Self::verify_rs256(modulus, exponent, message, signature)
            }
            (_, algorithm) => Err(CoseError::new(
                CoseErrorKind::UnsupportedAlgorithm,
                format!("no verification is implemented for the algorithm {algorithm}"),
            )),
        }
    }

    /// Verifies an ES256 signature over the SHA-256 of the message.
    fn verify_es256(
        x: &[u8],
        y: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), CoseError> {
        use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};

        let mut point = Vec::with_capacity(1 + x.len() + y.len());
        point.push(0x04);
        point.extend_from_slice(x);
        point.extend_from_slice(y);

        let key = VerifyingKey::from_sec1_bytes(&point).map_err(|error| {
            CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!("the point of the COSE key is not on P-256: {error}"),
            )
        })?;
        let signature = Signature::from_slice(signature).map_err(|error| {
            CoseError::new(
                CoseErrorKind::Signature,
                format!(
                    "the signature is {} bytes of ES256, which is not what the algorithm takes: \
                     {error}",
                    signature.len()
                ),
            )
        })?;

        key.verify(message, &signature).map_err(|_| {
            CoseError::new(
                CoseErrorKind::Signature,
                "the signature of the ceremony does not verify against the public key of the \
                 credential",
            )
        })
    }

    /// Verifies an RS256 signature over the SHA-256 of the message.
    fn verify_rs256(
        modulus: &[u8],
        exponent: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), CoseError> {
        use rsa::{
            pkcs1v15::{Signature, VerifyingKey},
            signature::Verifier,
            BigUint, RsaPublicKey,
        };
        use sha2::Sha256;

        let key = RsaPublicKey::new(
            BigUint::from_bytes_be(modulus),
            BigUint::from_bytes_be(exponent),
        )
        .map_err(|error| {
            CoseError::new(
                CoseErrorKind::UnsupportedKey,
                format!("the modulus and the exponent of the COSE key are not an RSA key: {error}"),
            )
        })?;
        let signature = Signature::try_from(signature).map_err(|error| {
            CoseError::new(
                CoseErrorKind::Signature,
                format!(
                    "the signature is {} bytes, which is not the length of the modulus: {error}",
                    signature.len()
                ),
            )
        })?;

        VerifyingKey::<Sha256>::new(key)
            .verify(message, &signature)
            .map_err(|_| {
                CoseError::new(
                    CoseErrorKind::Signature,
                    "the signature of the ceremony does not verify against the public key of the \
                     credential",
                )
            })
    }
}

/// What went wrong while a COSE key or a signature was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoseErrorKind {
    /// The bytes are not a COSE key, or not the COSE key they claim to be.
    Malformed,
    /// The key names an algorithm this port does not implement.
    UnsupportedAlgorithm,
    /// The key is of a type or a curve this port cannot use.
    UnsupportedKey,
    /// The signature does not verify against the key.
    Signature,
}

/// The error of reading a COSE key or verifying a signature with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoseError {
    kind: CoseErrorKind,
    message: String,
}

impl CoseError {
    /// Creates the error of the given kind with the given message.
    pub fn new(kind: CoseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns what went wrong.
    pub fn kind(&self) -> CoseErrorKind {
        self.kind
    }

    /// Returns the message of the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CoseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CoseError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webauthn::test_support::{
        base64_url, cbor_bytes, cbor_integer, cbor_map, es256_cose_key, rs256_cose_key,
        RFC_7515_A2_SIGNATURE, RFC_7515_A2_SIGNING_INPUT, RFC_7515_A3_SIGNATURE,
        RFC_7515_A3_SIGNING_INPUT, RFC_7515_A3_X, RFC_7515_A3_Y,
    };

    #[test]
    fn the_es256_key_of_the_rfc_is_read() {
        let key = CosePublicKey::parse(&es256_cose_key()).expect("the COSE key parses");

        assert_eq!(key.algorithm(), -7, "ES256");
        assert_eq!(key.curve(), Some(1), "P-256");
    }

    #[test]
    fn the_es256_signature_of_the_rfc_verifies() {
        let key = CosePublicKey::parse(&es256_cose_key()).expect("the COSE key parses");

        key.verify(
            RFC_7515_A3_SIGNING_INPUT.as_bytes(),
            &base64_url(RFC_7515_A3_SIGNATURE),
        )
        .expect("the signature is the one RFC 7515 appendix A.3 publishes");
    }

    #[test]
    fn a_tampered_es256_signature_is_refused() {
        let key = CosePublicKey::parse(&es256_cose_key()).expect("the COSE key parses");
        let mut signature = base64_url(RFC_7515_A3_SIGNATURE);
        signature[0] ^= 0x01;

        let error = key
            .verify(RFC_7515_A3_SIGNING_INPUT.as_bytes(), &signature)
            .expect_err("the signature of another value is not this one");

        assert_eq!(error.kind(), CoseErrorKind::Signature);
    }

    #[test]
    fn the_rs256_key_of_the_rfc_is_read() {
        let key = CosePublicKey::parse(&rs256_cose_key()).expect("the COSE key parses");

        assert_eq!(key.algorithm(), -257, "RS256");
        assert_eq!(key.curve(), None, "an RSA key has no curve");
    }

    #[test]
    fn the_rs256_signature_of_the_rfc_verifies() {
        let key = CosePublicKey::parse(&rs256_cose_key()).expect("the COSE key parses");

        key.verify(
            RFC_7515_A2_SIGNING_INPUT.as_bytes(),
            &base64_url(RFC_7515_A2_SIGNATURE),
        )
        .expect("the signature is the one RFC 7515 appendix A.2 publishes");
    }

    #[test]
    fn a_signature_over_another_message_is_refused() {
        let key = CosePublicKey::parse(&rs256_cose_key()).expect("the COSE key parses");

        let error = key
            .verify(
                b"another message",
                &base64_url(RFC_7515_A2_SIGNATURE),
            )
            .expect_err("the signature covers the signing input of the RFC");

        assert_eq!(error.kind(), CoseErrorKind::Signature);
    }

    #[test]
    fn an_algorithm_the_port_does_not_implement_is_refused_by_number() {
        let raw = cbor_map(&[
            (cbor_integer(1), cbor_integer(2)),
            (cbor_integer(3), cbor_integer(-8)),
            (cbor_integer(-1), cbor_integer(6)),
            (cbor_integer(-2), cbor_bytes(&base64_url(RFC_7515_A3_X))),
            (cbor_integer(-3), cbor_bytes(&base64_url(RFC_7515_A3_Y))),
        ]);

        let error = CosePublicKey::parse(&raw).expect_err("EdDSA is not implemented");

        assert_eq!(error.kind(), CoseErrorKind::UnsupportedAlgorithm);
        assert!(
            error.message().contains("-8"),
            "the error names the algorithm it cannot use, was: {error}"
        );
    }

    #[test]
    fn a_key_of_another_curve_is_refused() {
        let raw = cbor_map(&[
            (cbor_integer(1), cbor_integer(2)),
            (cbor_integer(3), cbor_integer(-7)),
            (cbor_integer(-1), cbor_integer(2)),
            (cbor_integer(-2), cbor_bytes(&[0x11; 48])),
            (cbor_integer(-3), cbor_bytes(&[0x22; 48])),
        ]);

        let error = CosePublicKey::parse(&raw).expect_err("P-384 is not implemented");

        assert_eq!(error.kind(), CoseErrorKind::UnsupportedKey);
        assert!(
            error.message().contains("P-384"),
            "the error names the curve it cannot use, was: {error}"
        );
    }

    #[test]
    fn a_key_without_an_algorithm_is_refused() {
        let raw = cbor_map(&[
            (cbor_integer(1), cbor_integer(2)),
            (cbor_integer(-1), cbor_integer(1)),
            (cbor_integer(-2), cbor_bytes(&base64_url(RFC_7515_A3_X))),
            (cbor_integer(-3), cbor_bytes(&base64_url(RFC_7515_A3_Y))),
        ]);

        let error = CosePublicKey::parse(&raw).expect_err("the algorithm is named by the key");

        assert_eq!(error.kind(), CoseErrorKind::UnsupportedAlgorithm);
    }

    #[test]
    fn a_cose_key_that_is_not_a_map_is_refused() {
        let error = CosePublicKey::parse(b"not a cose key").expect_err("this is not CBOR");

        assert_eq!(error.kind(), CoseErrorKind::Malformed);
    }

    #[test]
    fn the_key_reports_how_many_bytes_it_occupies() {
        let mut raw = es256_cose_key();
        let key_length = raw.len();
        raw.extend_from_slice(&cbor_map(&[]));

        let (key, consumed) = CosePublicKey::parse_prefix(&raw).expect("the key parses");

        assert_eq!(key.algorithm(), -7);
        assert_eq!(
            consumed, key_length,
            "the extensions that follow the key are not part of it"
        );
    }

    #[test]
    fn an_ec_key_of_the_wrong_length_is_refused() {
        let raw = cbor_map(&[
            (cbor_integer(1), cbor_integer(2)),
            (cbor_integer(3), cbor_integer(-7)),
            (cbor_integer(-1), cbor_integer(1)),
            (cbor_integer(-2), cbor_bytes(&[0x11; 16])),
            (cbor_integer(-3), cbor_bytes(&[0x22; 32])),
        ]);

        let error = CosePublicKey::parse(&raw).expect_err("the coordinates are not 32 bytes");

        assert_eq!(error.kind(), CoseErrorKind::UnsupportedKey);
    }
}
