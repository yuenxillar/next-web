//! The keys the tokens of the authorization server are signed with.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A JSON Web Key.
///
/// The key is the symmetric key the tokens are signed with. The identifier of
/// the key is part of the header of a token, so a resource server can look the
/// key up in the JWK set of the server.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Jwk {
    kty: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#use: Option<String>,
    k: String,
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

        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        let kid = hex::encode(&hasher.finalize()[..8]);

        Self {
            kty: "oct".to_string(),
            kid: Some(kid),
            alg: Some("HS256".to_string()),
            r#use: Some("sig".to_string()),
            k: URL_SAFE_NO_PAD.encode(secret.as_bytes()),
        }
    }

    /// Returns the identifier of the key.
    pub fn kid(&self) -> Option<&str> {
        self.kid.as_deref()
    }

    /// Returns the algorithm of the key.
    pub fn alg(&self) -> Option<&str> {
        self.alg.as_deref()
    }

    /// Returns the key material, decoded from its base64url form.
    pub fn key_material(&self) -> Vec<u8> {
        URL_SAFE_NO_PAD.decode(&self.k).unwrap_or_default()
    }
}

/// A set of JSON Web Keys.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
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

    /// Returns the first key of the set.
    pub fn find_by_kid(&self, kid: &str) -> Option<&Jwk> {
        self.keys.iter().find(|key| key.kid.as_deref() == Some(kid))
    }

    /// Returns the set as the JSON document of the JWK set endpoint.
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({ "keys": [] }))
    }
}

/// Provides the keys the tokens of the authorization server are signed with.
pub trait JwkSource: Send + Sync {
    /// Returns the JWK set of the authorization server.
    fn jwk_set(&self) -> JwkSet;
}

/// A [`JwkSource`] built from a single symmetric key.
#[derive(Clone, Debug)]
pub struct SecretJwkSource {
    jwk: Jwk,
}

impl SecretJwkSource {
    /// Creates a source from a secret.
    ///
    /// # Arguments
    ///
    /// * `secret` - The secret the tokens are signed with.
    pub fn new(secret: &str) -> Self {
        Self {
            jwk: Jwk::symmetric(secret),
        }
    }

    /// Returns the key of the source.
    pub fn jwk(&self) -> &Jwk {
        &self.jwk
    }
}

impl JwkSource for SecretJwkSource {
    fn jwk_set(&self) -> JwkSet {
        JwkSet::new(vec![self.jwk.clone()])
    }
}

/// Encodes the bytes with the base64url alphabet, without padding.
///
/// # Arguments
///
/// * `bytes` - The bytes to encode.
pub fn base64_url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Signs a message with HMAC SHA-256.
///
/// # Arguments
///
/// * `key` - The key of the signature.
/// * `message` - The message to sign.
pub fn sign_hs256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("an HMAC accepts a key of any size");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

/// Builds and signs a JSON Web Token.
///
/// # Arguments
///
/// * `jwk` - The key the token is signed with.
/// * `claims` - The claims of the token.
///
/// # Returns
///
/// The compact serialization of the token.
pub fn encode_jwt(jwk: &Jwk, claims: &Value) -> String {
    let mut header = serde_json::json!({
        "alg": jwk.alg().unwrap_or("HS256"),
        "typ": "JWT",
    });
    if let Some(kid) = jwk.kid() {
        header["kid"] = serde_json::json!(kid);
    }

    let header = base64_url(header.to_string().as_bytes());
    let payload = base64_url(claims.to_string().as_bytes());
    let signing_input = format!("{header}.{payload}");
    let signature = sign_hs256(&jwk.key_material(), signing_input.as_bytes());

    format!("{signing_input}.{}", base64_url(&signature))
}

/// Decodes the claims of a JSON Web Token without verifying its signature.
///
/// # Arguments
///
/// * `token` - The compact serialization of the token.
///
/// # Returns
///
/// The claims of the token, if it is a token of three parts.
pub fn decode_jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{decode_jwt_claims, encode_jwt, Jwk, JwkSet, JwkSource, SecretJwkSource};

    #[test]
    fn a_symmetric_key_is_derived_from_its_secret() {
        let key = Jwk::symmetric("secret");

        assert_eq!(key.kty, "oct");
        assert_eq!(key.alg(), Some("HS256"));
        assert_eq!(key.key_material(), b"secret");
        assert_eq!(key, Jwk::symmetric("secret"));
    }

    #[test]
    fn a_key_source_serves_the_set_of_its_key() {
        let source = SecretJwkSource::new("secret");

        let jwk_set = source.jwk_set();

        assert_eq!(jwk_set.keys().len(), 1);
        assert_eq!(
            JwkSet::new(jwk_set.keys().to_vec()).to_json()["keys"][0]["kty"],
            "oct"
        );
    }

    #[test]
    fn a_token_is_signed_and_carries_its_claims() {
        let key = Jwk::symmetric("secret");
        let token = encode_jwt(&key, &json!({ "sub": "user", "scope": "read" }));

        let parts = token.split('.').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);
        assert_eq!(decode_jwt_claims(&token).unwrap()["sub"], "user");
    }
}
