//! The base64url encoding every byte string of a ceremony travels in.
//!
//! The browser writes a credential id, a challenge, a public key and a signature
//! as base64url without padding, and the options the relying party hands back
//! have to be written the same way. Serde reaches the two directions through
//! this module: `#[serde(with = "crate::webauthn::base64url")]`.

use base64::Engine;
use serde::{Deserialize, Deserializer, Serializer};

/// Writes the bytes as a base64url string without padding.
pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

/// Reads a base64url string back into bytes.
///
/// Only the canonical form is accepted: a string that carries padding, or that
/// re-encodes to something else, is refused rather than silently normalised.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    decode(&value).map_err(serde::de::Error::custom)
}

/// Decodes a canonical base64url string without padding.
pub fn decode(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let raw = engine.decode(value)?;
    if engine.encode(&raw) != value {
        return Err(base64::DecodeError::InvalidLength(value.len()));
    }

    Ok(raw)
}
