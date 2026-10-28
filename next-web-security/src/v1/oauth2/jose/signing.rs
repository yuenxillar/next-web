//! The signers an authorization server signs its tokens with.

use std::sync::Arc;

use hmac::{Hmac, Mac};
use rsa::{
    pkcs1v15::SigningKey,
    signature::{RandomizedSigner, SignatureEncoding},
    RsaPrivateKey, RsaPublicKey,
};
use serde_json::Value;
use sha2::Sha256;

use crate::oauth2::{
    jose::{base64_url, Jwk, JwkSet, JwkSource, HS256},
    server::authorization::error::OAuth2Error,
};

/// Signs the tokens of an authorization server.
///
/// A signer publishes the keys it signs with, so the resource servers that
/// verify the tokens can read them.
pub trait JwtSigner: JwkSource {
    /// Signs the claims and returns the compact serialization of the token.
    ///
    /// # Arguments
    ///
    /// * `claims` - The claims of the token.
    fn sign(&self, claims: &Value) -> Result<String, OAuth2Error>;
}

/// A [`JwtSigner`] that signs its tokens with a symmetric secret.
///
/// The secret is also the key that is published, so the signer is only suitable
/// for a deployment in which the resource servers can be trusted with the
/// secret. Use [`RsaJwtSigner`] otherwise.
#[derive(Clone, Debug)]
pub struct SecretJwtSigner {
    jwk: Jwk,
}

impl SecretJwtSigner {
    /// Creates a signer from a secret.
    ///
    /// # Arguments
    ///
    /// * `secret` - The secret the tokens are signed with.
    pub fn new(secret: &str) -> Self {
        Self {
            jwk: Jwk::symmetric(secret),
        }
    }

    /// Returns the key of the signer.
    pub fn jwk(&self) -> &Jwk {
        &self.jwk
    }
}

impl JwkSource for SecretJwtSigner {
    fn jwk_set(&self) -> JwkSet {
        JwkSet::new(vec![self.jwk.clone()])
    }
}

impl JwtSigner for SecretJwtSigner {
    fn sign(&self, claims: &Value) -> Result<String, OAuth2Error> {
        let key = self.jwk.secret().ok_or_else(|| {
            OAuth2Error::server_error("The symmetric key of the server has no secret")
        })?;
        let header = header(&self.jwk);
        let signing_input = format!(
            "{}.{}",
            base64_url(header.to_string().as_bytes()),
            base64_url(claims.to_string().as_bytes())
        );

        let mut mac = Hmac::<Sha256>::new_from_slice(&key)
            .map_err(|_| OAuth2Error::server_error("The symmetric key is invalid"))?;
        mac.update(signing_input.as_bytes());
        let signature = base64_url(&mac.finalize().into_bytes());

        Ok(format!("{signing_input}.{signature}"))
    }
}

/// A [`JwtSigner`] that signs its tokens with an RSA key pair.
///
/// Only the public part of the key pair is published, which is what a
/// deployment that issues tokens for other services uses.
pub struct RsaJwtSigner {
    private_key: RsaPrivateKey,
    public_jwk: Jwk,
}

impl RsaJwtSigner {
    /// Generates an RSA key pair of the given size.
    ///
    /// # Arguments
    ///
    /// * `bits` - The size of the key, at least 2048 bits.
    pub fn generate(bits: usize) -> Result<Self, OAuth2Error> {
        assert!(bits >= 2048, "an RSA key of at least 2048 bits is required");

        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, bits).map_err(|error| {
            OAuth2Error::server_error(format!("The key is not generated: {error}"))
        })?;

        Ok(Self::from_private_key(private_key))
    }

    /// Creates a signer from a private key.
    ///
    /// # Arguments
    ///
    /// * `private_key` - The private part of the key pair.
    pub fn from_private_key(private_key: RsaPrivateKey) -> Self {
        let public_key = RsaPublicKey::from(&private_key);

        Self {
            private_key,
            public_jwk: Jwk::rsa(&public_key),
        }
    }

    /// Creates a signer from the PEM form of a private key.
    ///
    /// # Arguments
    ///
    /// * `pem` - The PEM of the private key.
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self, OAuth2Error> {
        use rsa::pkcs8::DecodePrivateKey;

        let private_key = RsaPrivateKey::from_pkcs8_pem(pem)
            .map_err(|error| OAuth2Error::server_error(format!("The key is not read: {error}")))?;

        Ok(Self::from_private_key(private_key))
    }

    /// Returns the public key of the signer.
    pub fn public_jwk(&self) -> &Jwk {
        &self.public_jwk
    }
}

impl JwkSource for RsaJwtSigner {
    fn jwk_set(&self) -> JwkSet {
        JwkSet::new(vec![self.public_jwk.clone()])
    }
}

impl JwtSigner for RsaJwtSigner {
    fn sign(&self, claims: &Value) -> Result<String, OAuth2Error> {
        let header = header(&self.public_jwk);
        let signing_input = format!(
            "{}.{}",
            base64_url(header.to_string().as_bytes()),
            base64_url(claims.to_string().as_bytes())
        );

        let signing_key = SigningKey::<Sha256>::new(self.private_key.clone());
        let mut rng = rand::thread_rng();
        let signature = signing_key.sign_with_rng(&mut rng, signing_input.as_bytes());

        Ok(format!(
            "{signing_input}.{}",
            base64_url(&signature.to_bytes())
        ))
    }
}

/// Wraps a signer so that it can be used where only the keys are needed.
#[derive(Clone)]
pub struct SignerJwkSource {
    signer: Arc<dyn JwtSigner>,
}

impl SignerJwkSource {
    /// Creates a key source from a signer.
    ///
    /// # Arguments
    ///
    /// * `signer` - The signer the keys are read from.
    pub fn new(signer: Arc<dyn JwtSigner>) -> Self {
        Self { signer }
    }
}

impl JwkSource for SignerJwkSource {
    fn jwk_set(&self) -> JwkSet {
        self.signer.jwk_set()
    }
}

/// Builds the header of a token that is signed with the given key.
///
/// # Arguments
///
/// * `jwk` - The key the token is signed with.
fn header(jwk: &Jwk) -> Value {
    let mut header = serde_json::json!({
        "alg": jwk.alg().unwrap_or(HS256),
        "typ": "JWT",
    });
    if let Some(kid) = jwk.kid() {
        header["kid"] = serde_json::json!(kid);
    }
    header
}

#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use serde_json::json;

    use super::{
        Jwk, JwkSet, JwkSource, JwtSigner, RsaJwtSigner, SecretJwtSigner, SignerJwkSource,
    };
    use crate::oauth2::jose::{base64_url_decode, HS256, RS256};

    #[test]
    fn a_symmetric_signer_signs_its_claims() {
        let signer = SecretJwtSigner::new("secret");
        let token = signer
            .sign(&json!({ "sub": "user", "scope": "read" }))
            .expect("the token is signed");

        let parts = token.split('.').collect::<Vec<_>>();
        assert_eq!(parts.len(), 3);
        let payload = base64_url_decode(parts[1]).expect("the payload is base64url");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&payload).unwrap()["sub"],
            "user"
        );
        assert_eq!(signer.jwk_set().keys()[0].alg(), Some(HS256));
    }

    #[test]
    fn an_rsa_signer_signs_its_claims_with_the_identifier_of_its_key() {
        let signer = RsaJwtSigner::generate(2048).expect("the key is generated");
        let token = signer
            .sign(&json!({ "sub": "user" }))
            .expect("the token is signed");

        let parts = token.split('.').collect::<Vec<_>>();
        let header = base64_url_decode(parts[0]).expect("the header is base64url");
        let header = serde_json::from_slice::<serde_json::Value>(&header).unwrap();

        assert_eq!(header["alg"], RS256);
        assert_eq!(header["kid"], signer.public_jwk().kid().unwrap());
        assert!(!parts[2].is_empty(), "the token carries its signature");
        assert!(
            URL_SAFE_NO_PAD.decode(parts[2]).is_ok(),
            "the signature is base64url"
        );
    }

    #[test]
    fn a_wrapped_signer_publishes_the_keys_of_the_signer() {
        let signer = std::sync::Arc::new(SecretJwtSigner::new("secret"));
        let source = SignerJwkSource::new(signer);

        assert_eq!(source.jwk_set().keys().len(), 1);
        assert_eq!(
            JwkSet::new(source.jwk_set().keys().to_vec()).to_json()["keys"][0]["kty"],
            "oct"
        );
    }

    #[test]
    fn an_rsa_key_publishes_only_the_public_part() {
        let signer = RsaJwtSigner::generate(2048).expect("the key is generated");
        let jwk_set = signer.jwk_set();
        let jwk: &Jwk = jwk_set.keys().first().expect("the set holds the key");

        assert_eq!(jwk.kty(), "RSA");
        assert_eq!(jwk.alg(), Some(RS256));
        assert!(jwk.modulus().is_some());
        assert!(jwk.exponent().is_some());
        assert!(jwk.secret().is_none(), "the private part is not published");
        assert!(jwk_set.to_json()["keys"][0].get("k").is_none());
    }
}
