//! The keys an authorization server signs its tokens with, and the keys a
//! resource server verifies them with.
//!
//! The key model and the key sources are shared by the two sides: the JWK set
//! endpoint publishes the public keys of a server, and the decoder of a
//! resource server reads them. The signing of a token belongs to the server, so
//! it is only part of a build that supports the authorization server.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rsa::{traits::PublicKeyParts, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[cfg(feature = "oauth2-authorization-server")]
mod signing;
#[cfg(feature = "oauth2-authorization-server")]
pub use signing::{JwtSigner, RsaJwtSigner, SecretJwtSigner, SignerJwkSource};

/// The algorithm of a key that is a symmetric secret.
pub const HS256: &str = "HS256";

/// The algorithm of a key that is an RSA key pair.
pub const RS256: &str = "RS256";

/// A JSON Web Key.
///
/// A key of an authorization server is either the symmetric secret the tokens
/// are signed with (`oct`), or the public part of an RSA key pair (`RSA`). The
/// private part of an RSA key is never published, so a resource server can only
/// verify the tokens and cannot issue them.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct Jwk {
    kty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    k: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    e: Option<String>,
}

impl Jwk {
    /// Creates a symmetric key from a secret.
    ///
    /// The identifier of the key is derived from the secret, so the same secret
    /// always produces the same key.
    ///
    /// # Arguments
    ///
    /// * `secret` - The secret of the key.
    pub fn symmetric(secret: &str) -> Self {
        assert!(!secret.is_empty(), "the secret of a key cannot be empty");

        Self {
            kty: "oct".to_string(),
            kid: Some(key_id(secret.as_bytes())),
            alg: Some(HS256.to_string()),
            r#use: Some("sig".to_string()),
            k: Some(URL_SAFE_NO_PAD.encode(secret.as_bytes())),
            n: None,
            e: None,
        }
    }

    /// Creates the public key of an RSA key pair.
    ///
    /// # Arguments
    ///
    /// * `public_key` - The public part of the key pair.
    pub fn rsa(public_key: &RsaPublicKey) -> Self {
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

        Self {
            kty: "RSA".to_string(),
            kid: Some(key_id(&public_key.n().to_bytes_be())),
            alg: Some(RS256.to_string()),
            r#use: Some("sig".to_string()),
            k: None,
            n: Some(n),
            e: Some(e),
        }
    }

    /// Returns the type of the key.
    pub fn kty(&self) -> &str {
        &self.kty
    }

    /// Returns the identifier of the key.
    pub fn kid(&self) -> Option<&str> {
        self.kid.as_deref()
    }

    /// Returns the algorithm of the key.
    pub fn alg(&self) -> Option<&str> {
        self.alg.as_deref()
    }

    /// Returns the secret of a symmetric key, decoded from its base64url form.
    pub fn secret(&self) -> Option<Vec<u8>> {
        self.k.as_ref().and_then(|k| URL_SAFE_NO_PAD.decode(k).ok())
    }

    /// Returns the modulus of an RSA key.
    pub fn modulus(&self) -> Option<&str> {
        self.n.as_deref()
    }

    /// Returns the exponent of an RSA key.
    pub fn exponent(&self) -> Option<&str> {
        self.e.as_deref()
    }

    /// Sets the identifier of the key.
    ///
    /// # Arguments
    ///
    /// * `kid` - The identifier of the key.
    pub fn set_kid(&mut self, kid: impl Into<String>) {
        self.kid = Some(kid.into());
    }
}

/// A set of JSON Web Keys.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct JwkSet {
    keys: Vec<Jwk>,
}

impl JwkSet {
    /// Creates a set with the given keys.
    ///
    /// # Arguments
    ///
    /// * `keys` - The keys of the set.
    pub fn new(keys: Vec<Jwk>) -> Self {
        Self { keys }
    }

    /// Returns the keys of the set.
    pub fn keys(&self) -> &[Jwk] {
        &self.keys
    }

    /// Returns the key with the given identifier.
    ///
    /// # Arguments
    ///
    /// * `kid` - The identifier of the key.
    pub fn find_by_kid(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|key| key.kid.as_deref() == Some(kid))
    }

    /// Returns the first key of the set.
    pub fn first(&self) -> Option<&Jwk> {
        self.keys.first()
    }

    /// Returns the set as the JSON document of the JWK set endpoint.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({ "keys": [] }))
    }
}

/// Provides the public keys of an authorization server.
pub trait JwkSource: Send + Sync {
    /// Returns the keys of the server, without any private material.
    fn jwk_set(&self) -> JwkSet;
}

/// Encodes the bytes with the base64url alphabet, without padding.
///
/// # Arguments
///
/// * `bytes` - The bytes to encode.
pub fn base64_url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Decodes a base64url value.
///
/// # Arguments
///
/// * `value` - The value to decode.
pub fn base64_url_decode(value: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(value).ok()
}

/// Returns the identifier of a key, derived from its material.
///
/// # Arguments
///
/// * `material` - The material of the key.
fn key_id(material: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(material);
    hex::encode(&hasher.finalize()[..8])
}

#[cfg(test)]
mod tests {
    use super::{Jwk, JwkSet, HS256, RS256};

    #[test]
    fn a_symmetric_key_is_derived_from_its_secret() {
        let key = Jwk::symmetric("secret");

        assert_eq!(key.kty(), "oct");
        assert_eq!(key.alg(), Some(HS256));
        assert_eq!(key.secret().as_deref(), Some(b"secret".as_slice()));
        assert_eq!(key, Jwk::symmetric("secret"));
    }

    #[test]
    fn a_key_set_is_read_from_the_document_of_a_server() {
        let document = serde_json::json!({
            "keys": [
                { "kty": "RSA", "kid": "the-key", "alg": RS256, "use": "sig", "n": "modulus", "e": "AQAB" },
                { "kty": "oct", "kid": "secret", "alg": HS256, "k": "c2VjcmV0" },
            ]
        });

        let jwk_set: JwkSet =
            serde_json::from_value(document).expect("the JWK set of a server is read");

        assert_eq!(jwk_set.keys().len(), 2);
        let rsa = jwk_set.find_by_kid("the-key").expect("the RSA key is read");
        assert_eq!(rsa.kty(), "RSA");
        assert_eq!(rsa.modulus(), Some("modulus"));
        let symmetric = jwk_set.find_by_kid("secret").expect("the secret is read");
        assert_eq!(symmetric.secret().as_deref(), Some(b"secret".as_slice()));
    }
}
