//! The validation of the identifier token of an OpenID Connect grant.

use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use next_web_core::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind},
    oauth2::{
        jose::{base64_url, base64_url_decode, JwkSet},
        ClientRegistration,
    },
};

/// The tolerated clock skew of the registered claims, in seconds.
const LEEWAY: u64 = 60;

/// An identifier token that was validated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OidcIdToken {
    issuer: String,
    subject: String,
    audience: Vec<String>,
    claims: Value,
}

impl OidcIdToken {
    /// Creates a validated identifier token.
    ///
    /// # Arguments
    ///
    /// * `issuer` - The issuer the token names.
    /// * `subject` - The subject the token names.
    /// * `audience` - The audiences the token names.
    /// * `claims` - The claims of the token.
    pub fn new(
        issuer: impl Into<String>,
        subject: impl Into<String>,
        audience: Vec<String>,
        claims: Value,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            subject: subject.into(),
            audience,
            claims,
        }
    }

    /// Returns the issuer the token names.
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Returns the subject the token names.
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the audiences the token names.
    pub fn audience(&self) -> &[String] {
        &self.audience
    }

    /// Returns the claims of the token.
    pub fn claims(&self) -> &Value {
        &self.claims
    }
}

/// Validates the identifier token of an OpenID Connect grant.
///
/// The token is refused when it cannot be verified, which is what keeps a
/// client from trusting a token it cannot tell apart from a token an attacker
/// made up.
#[async_trait]
pub trait OidcIdTokenValidator: Send + Sync {
    /// Validates an identifier token.
    ///
    /// # Arguments
    ///
    /// * `client_registration` - The client the grant was issued to.
    /// * `id_token` - The identifier token of the token response.
    /// * `nonce` - The nonce of the authorization request, if one was sent.
    /// * `access_token` - The access token of the grant, if the token carries
    ///   the hash of one.
    async fn validate(
        &self,
        client_registration: &ClientRegistration,
        id_token: &str,
        nonce: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<OidcIdToken, AuthenticationError>;
}

/// The default validator of an identifier token.
///
/// The signature of the token is verified with the keys of the JWK set of the
/// provider, and the registered claims of the specification are verified: the
/// issuer, the audience, the subject, the expiry, the `azp`, the nonce of the
/// authorization request, and the `at_hash` of the access token.
///
/// A provider whose keys the client cannot read (a registration that does not
/// name the JWK set of the provider) is refused, because a client that cannot
/// verify a token cannot trust it.
#[derive(Clone)]
pub struct DefaultOidcIdTokenValidator {
    http_client: reqwest::Client,
}

impl Default for DefaultOidcIdTokenValidator {
    fn default() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }
}

impl DefaultOidcIdTokenValidator {
    /// Creates a validator that reads the keys of the provider with its own
    /// HTTP client.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a validator that reads the keys of the provider with the given
    /// HTTP client.
    ///
    /// # Arguments
    ///
    /// * `http_client` - The HTTP client of the validator.
    pub fn with_http_client(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }

    /// Reads the keys of the provider.
    ///
    /// # Arguments
    ///
    /// * `jwk_set_uri` - The URI of the JWK set of the provider.
    async fn jwk_set(&self, jwk_set_uri: &str) -> Result<JwkSet, AuthenticationError> {
        let response = self
            .http_client
            .get(jwk_set_uri)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|error| {
                invalid_id_token(format!(
                    "The JWK set of the provider is not reached: {error}"
                ))
            })?;
        if !response.status().is_success() {
            return Err(invalid_id_token(format!(
                "The JWK set of the provider reported {}",
                response.status()
            )));
        }

        response.json::<JwkSet>().await.map_err(|error| {
            invalid_id_token(format!(
                "The JWK set of the provider is not a valid document: {error}"
            ))
        })
    }
}

#[async_trait]
impl OidcIdTokenValidator for DefaultOidcIdTokenValidator {
    async fn validate(
        &self,
        client_registration: &ClientRegistration,
        id_token: &str,
        nonce: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<OidcIdToken, AuthenticationError> {
        let jwk_set_uri = client_registration.jwk_set_uri().ok_or_else(|| {
            invalid_id_token(
                "The client registration does not name the JWK set of the provider, so the \
                 identifier token cannot be verified",
            )
        })?;

        let header = jsonwebtoken::decode_header(id_token)
            .map_err(|_| invalid_id_token("The identifier token is not a well-formed JWT"))?;
        if unverified_claims(id_token).is_none() {
            return Err(invalid_id_token(
                "The identifier token has no claims to read",
            ));
        }

        let jwk_set = self.jwk_set(jwk_set_uri).await?;
        let decoding_key = decoding_key(&jwk_set, &header)?;

        let mut validation = Validation::new(header.alg);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = LEEWAY;
        // The audience and the issuer are verified below, where the value of a
        // claim can be reported in the message of the error.
        validation.validate_aud = false;

        let decoded = jsonwebtoken::decode::<Value>(id_token, &decoding_key, &validation).map_err(
            |error| invalid_id_token(format!("The identifier token is not valid: {error}")),
        )?;

        validate_claims(&decoded.claims, client_registration, nonce, access_token)
    }
}

/// Verifies the registered claims of an identifier token.
///
/// # Arguments
///
/// * `claims` - The claims of the token.
/// * `client_registration` - The client the grant was issued to.
/// * `nonce` - The nonce of the authorization request, if one was sent.
/// * `access_token` - The access token of the grant.
fn validate_claims(
    claims: &Value,
    client_registration: &ClientRegistration,
    nonce: Option<&str>,
    access_token: Option<&str>,
) -> Result<OidcIdToken, AuthenticationError> {
    let issuer = claims
        .get("iss")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_id_token("The identifier token does not name its issuer"))?;
    if let Some(expected_issuer) = client_registration.issuer_uri() {
        if issuer != expected_issuer {
            return Err(invalid_id_token(format!(
                "The identifier token names the issuer {issuer}, but the client trusts \
                 {expected_issuer}"
            )));
        }
    }

    let subject = claims
        .get("sub")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_id_token("The identifier token does not name its subject"))?;

    let audience = audience_of(claims)
        .ok_or_else(|| invalid_id_token("The identifier token does not name its audience"))?;
    if !audience
        .iter()
        .any(|audience| audience == client_registration.client_id())
    {
        return Err(invalid_id_token(format!(
            "The identifier token is not issued to the client {}",
            client_registration.client_id()
        )));
    }

    // The token names the client it was issued to with `azp` when more than one
    // audience is named, so an authorized party has to be the client.
    if audience.len() > 1 || claims.get("azp").is_some() {
        let authorized_party = claims.get("azp").and_then(Value::as_str).ok_or_else(|| {
            invalid_id_token(
                "The identifier token names more than one audience without naming the \
                     authorized party",
            )
        })?;
        if authorized_party != client_registration.client_id() {
            return Err(invalid_id_token(format!(
                "The identifier token was authorized by {authorized_party}, not by the client {}",
                client_registration.client_id()
            )));
        }
    }

    if let Some(nonce) = nonce {
        let token_nonce = claims.get("nonce").and_then(Value::as_str);
        if token_nonce != Some(nonce) {
            return Err(invalid_id_token(
                "The identifier token does not carry the nonce of the authorization request",
            ));
        }
    }

    if let Some(access_token_hash) = claims.get("at_hash").and_then(Value::as_str) {
        let access_token = access_token.ok_or_else(|| {
            invalid_id_token(
                "The identifier token carries the hash of an access token, but the grant has none",
            )
        })?;
        if access_token_hash != access_token_hash_of(access_token) {
            return Err(invalid_id_token(
                "The identifier token does not carry the hash of the access token of the grant",
            ));
        }
    }

    Ok(OidcIdToken {
        issuer: issuer.to_string(),
        subject: subject.to_string(),
        audience,
        claims: claims.clone(),
    })
}

/// Returns the audiences of a document.
///
/// # Arguments
///
/// * `claims` - The claims of the token.
fn audience_of(claims: &Value) -> Option<Vec<String>> {
    match claims.get("aud")? {
        Value::String(audience) => Some(vec![audience.clone()]),
        Value::Array(audiences) => Some(
            audiences
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect(),
        ),
        _ => None,
    }
}

/// Returns the `at_hash` of an access token.
///
/// The hash is the base64url encoding of the left half of the SHA-256 digest of
/// the value of the token.
///
/// # Arguments
///
/// * `access_token` - The access token of the grant.
pub fn access_token_hash_of(access_token: &str) -> String {
    let digest = Sha256::digest(access_token.as_bytes());
    base64_url(&digest[..digest.len() / 2])
}

/// Builds the key that verifies an identifier token.
///
/// # Arguments
///
/// * `jwk_set` - The keys of the provider.
/// * `header` - The header of the token.
fn decoding_key(
    jwk_set: &JwkSet,
    header: &jsonwebtoken::Header,
) -> Result<DecodingKey, AuthenticationError> {
    let jwk = match header.kid.as_deref() {
        Some(kid) => jwk_set.find_by_kid(kid),
        None => jwk_set.first(),
    }
    .ok_or_else(|| invalid_id_token("The key of the identifier token is unknown"))?;

    let compatible = match jwk.kty() {
        "oct" => matches!(
            header.alg,
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512
        ),
        "RSA" => matches!(
            header.alg,
            Algorithm::RS256
                | Algorithm::RS384
                | Algorithm::RS512
                | Algorithm::PS256
                | Algorithm::PS384
                | Algorithm::PS512
        ),
        _ => false,
    };
    if !compatible {
        return Err(invalid_id_token(
            "The algorithm of the identifier token does not match the key of the provider",
        ));
    }

    match jwk.kty() {
        "oct" => jwk
            .secret()
            .map(|secret| DecodingKey::from_secret(&secret))
            .ok_or_else(|| invalid_id_token("The key of the provider is invalid")),
        "RSA" => {
            let (Some(modulus), Some(exponent)) = (jwk.modulus(), jwk.exponent()) else {
                return Err(invalid_id_token("The key of the provider is incomplete"));
            };
            DecodingKey::from_rsa_components(modulus, exponent)
                .map_err(|_| invalid_id_token("The key of the provider is invalid"))
        }
        _ => Err(invalid_id_token(
            "The kind of the key of the provider is not supported",
        )),
    }
}

/// Returns the claims of a token without verifying its signature.
///
/// The claims are read to name the issuer and the subject of a token the
/// signature of which is verified with the key the header names, so they are
/// not trusted on their own.
///
/// # Arguments
///
/// * `token` - The token to read.
fn unverified_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64_url_decode(payload)?;

    serde_json::from_slice(&bytes).ok()
}

/// Returns the error an identifier token that cannot be trusted reports.
///
/// # Arguments
///
/// * `message` - The reason the token is refused.
fn invalid_id_token(message: impl Into<String>) -> AuthenticationError {
    AuthenticationError::with_kind(message, AuthenticationErrorKind::BadCredentials)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        access_token_hash_of, unverified_claims, validate_claims, DefaultOidcIdTokenValidator,
        OidcIdTokenValidator,
    };
    use crate::oauth2::jose::JwtSigner;
    use crate::oauth2::{jose::SecretJwtSigner, ClientRegistration};

    /// Returns a registration of a client of a provider.
    fn registration() -> ClientRegistration {
        ClientRegistration::new("demo", "Demo", "https://provider.example/oauth2/authorize")
            .set_client_id("demo-client")
            .set_issuer_uri("https://provider.example")
    }

    #[test]
    fn a_valid_token_is_accepted() {
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": "demo-client",
            "exp": 4_102_444_800_u64,
        });

        let token = validate_claims(&claims, &registration(), None, None)
            .expect("the claims of the token are valid");

        assert_eq!(token.issuer(), "https://provider.example");
        assert_eq!(token.subject(), "user");
        assert_eq!(token.audience(), ["demo-client"]);
    }

    #[test]
    fn a_token_of_another_client_is_refused() {
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": "other-client",
        });

        assert!(validate_claims(&claims, &registration(), None, None).is_err());
    }

    #[test]
    fn a_token_of_another_issuer_is_refused() {
        let claims = json!({
            "iss": "https://other.example",
            "sub": "user",
            "aud": "demo-client",
        });

        assert!(validate_claims(&claims, &registration(), None, None).is_err());
    }

    #[test]
    fn a_token_with_another_nonce_is_refused() {
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": "demo-client",
            "nonce": "the-nonce",
        });

        assert!(validate_claims(&claims, &registration(), Some("the-nonce"), None).is_ok());
        assert!(validate_claims(&claims, &registration(), Some("other-nonce"), None).is_err());
        // A request that sent a nonce is refused when the token has none.
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": "demo-client",
        });
        assert!(validate_claims(&claims, &registration(), Some("the-nonce"), None).is_err());
    }

    #[test]
    fn a_token_with_the_hash_of_another_access_token_is_refused() {
        let access_token = "the-access-token";
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": "demo-client",
            "at_hash": access_token_hash_of(access_token),
        });

        assert!(validate_claims(&claims, &registration(), None, Some(access_token)).is_ok());
        assert!(validate_claims(&claims, &registration(), None, Some("another-token")).is_err());
    }

    #[test]
    fn a_token_of_more_than_one_audience_needs_the_client_as_its_authorized_party() {
        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": ["demo-client", "other-client"],
        });
        assert!(validate_claims(&claims, &registration(), None, None).is_err());

        let claims = json!({
            "iss": "https://provider.example",
            "sub": "user",
            "aud": ["demo-client", "other-client"],
            "azp": "demo-client",
        });
        assert!(validate_claims(&claims, &registration(), None, None).is_ok());
    }

    #[tokio::test]
    async fn a_registration_without_the_keys_of_the_provider_is_refused() {
        let validator = DefaultOidcIdTokenValidator::default();
        let token = SecretJwtSigner::new("secret")
            .sign(&json!({ "iss": "https://provider.example", "sub": "user" }))
            .expect("the token is signed");

        let error = validator
            .validate(&registration(), &token, None, None)
            .await
            .expect_err("the token cannot be verified without the keys of the provider");

        assert!(error.message().contains("does not name the JWK set"));
    }

    #[test]
    fn a_token_that_is_not_a_jwt_has_no_claims_to_read() {
        assert_eq!(unverified_claims("not-a-token"), None);
    }
}
