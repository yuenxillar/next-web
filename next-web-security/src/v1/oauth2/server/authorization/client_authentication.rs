//! Authenticates the client of a request that reaches a protected endpoint.

use base64::{engine::general_purpose::STANDARD, Engine};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use next_web_core::any::AnyValue;
use next_web_core::http::form_parameters;
use next_web_core::http::HttpRequest;
use serde_json::Value;

use super::{
    client::{
        client_not_found, client_secret_matches, ClientAuthenticationMethod, RegisteredClient,
        CLIENT_ASSERTION_TYPE_JWT_BEARER,
    },
    context::AuthorizationServerContext,
    error::OAuth2Error,
};
use crate::oauth2::jose::JwkSet;

/// The name of the request attribute that carries the parameters of a pushed
/// authorization request.
///
/// The authorization endpoint serves a request that names a request URI with
/// the parameters the client pushed, and the parameters are handed to the
/// endpoint as this attribute.
pub const PUSHED_AUTHORIZATION_REQUEST_ATTRIBUTE: &str =
    "next_web_security.pushed_authorization_request";

/// Returns the value of a form parameter of the request.
///
/// The parameters of a pushed authorization request are looked up first, then
/// the query of the request, and then the url-encoded body of the request. The
/// body is only available when the web layer buffered it, which it does for a
/// form post.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
/// * `name` - The name of the parameter.
pub fn form_parameter(request: &dyn HttpRequest, name: &str) -> Option<String> {
    if let Some(parameters) = request
        .get_attribute(PUSHED_AUTHORIZATION_REQUEST_ATTRIBUTE)
        .and_then(AnyValue::as_map)
    {
        if let Some(value) = parameters.get(name).and_then(AnyValue::as_str) {
            return Some(value.to_string());
        }
    }

    if let Some(value) = request.parameter(name) {
        return Some(decode(value));
    }

    let body = request.body_bytes()?;
    form_parameters(body)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// Returns every parameter of a request, from its query and its url-encoded
/// body.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
pub fn request_parameters(request: &dyn HttpRequest) -> Vec<(String, String)> {
    let mut parameters: Vec<(String, String)> = request
        .parameters()
        .map(|parameters| {
            parameters
                .into_iter()
                .map(|(name, value)| (name.into_owned(), value.into_owned()))
                .collect()
        })
        .unwrap_or_default();
    parameters.extend(form_parameter_map(request));

    parameters
}

/// Returns the parameters of the url-encoded body of the request.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
pub fn form_parameter_map(request: &dyn HttpRequest) -> Vec<(String, String)> {
    request
        .body_bytes()
        .map(|body| {
            form_parameters(body)
                .into_iter()
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// Authenticates the client of a request.
///
/// The client authenticates with the `Authorization` header
/// (`client_secret_basic`) or with the form parameters of the request
/// (`client_secret_post`), and a public client sends only its identifier.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
/// * `context` - The context of the authorization server.
pub fn authenticate_client(
    request: &dyn HttpRequest,
    context: &AuthorizationServerContext,
) -> Result<RegisteredClient, OAuth2Error> {
    // A client that proves its identity with a signed assertion.
    if let Some(assertion) = form_parameter(request, "client_assertion") {
        let assertion_type = form_parameter(request, "client_assertion_type").unwrap_or_default();
        if assertion_type != CLIENT_ASSERTION_TYPE_JWT_BEARER {
            return Err(OAuth2Error::invalid_client(
                "The type of the client assertion is not supported",
            ));
        }
        let client_id = form_parameter(request, "client_id");

        return authenticate_client_assertion(&assertion, client_id.as_deref(), context);
    }

    // A client that proves its identity with the certificate of a mutual TLS
    // connection. The connection is terminated in front of the application
    // while the application itself does not terminate it, so the subject of the
    // certificate is read from the header the terminator sets, and only when
    // the application trusts that header.
    if context.settings().trust_forwarded_client_certificate() {
        if let (Some(client_id), Some(subject)) = (
            form_parameter(request, "client_id"),
            tls_client_certificate_subject(request),
        ) {
            return authenticate_tls_client(&client_id, &subject, context);
        }
    }

    let credentials = client_credentials(request);

    let (client_id, presented_secret, method) = match credentials {
        Some(credentials) => credentials,
        None => {
            return Err(OAuth2Error::invalid_client(
                "Client authentication is required",
            ));
        }
    };

    let registered_client = context
        .registered_clients()
        .find_by_client_id(&client_id)
        .ok_or_else(|| client_not_found(&client_id))?;

    if !registered_client
        .client_authentication_methods()
        .contains(&method)
    {
        return Err(OAuth2Error::invalid_client(format!(
            "The client is not registered for the {} authentication method",
            method.value()
        )));
    }

    match (registered_client.client_secret(), presented_secret) {
        (Some(stored), Some(presented)) => {
            if !client_secret_matches(&presented, stored, context.password_encoder()) {
                return Err(OAuth2Error::invalid_client("The client secret is invalid"));
            }
        }
        (Some(_), None) => {
            return Err(OAuth2Error::invalid_client(
                "Client authentication is required",
            ));
        }
        (None, _) => {}
    }

    Ok(registered_client)
}

/// Returns whether the request carries client credentials.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
pub fn has_client_credentials(request: &dyn HttpRequest) -> bool {
    request
        .header("Authorization")
        .is_some_and(|header| header.starts_with("Basic "))
        || form_parameter(request, "client_id").is_some()
}

/// Authenticates a client that proves its identity with a signed assertion.
///
/// The assertion is a JWT that the client signs with its own key. The
/// identifier of the client is the `sub` claim of the assertion, which has to
/// agree with the `iss` claim and with the `client_id` of the request when the
/// request names one. The assertion has to be signed with a key of the client
/// and it has to name the issuer of the server as its audience.
///
/// # Arguments
///
/// * `assertion` - The JWT the client presented.
/// * `client_id` - The identifier of the request, if it names one.
/// * `context` - The context of the authorization server.
fn authenticate_client_assertion(
    assertion: &str,
    client_id: Option<&str>,
    context: &AuthorizationServerContext,
) -> Result<RegisteredClient, OAuth2Error> {
    let header = jsonwebtoken::decode_header(assertion)
        .map_err(|_| OAuth2Error::invalid_client("The client assertion is not a well-formed JWT"))?;
    let assertion_claims = unverified_claims(assertion)
        .ok_or_else(|| OAuth2Error::invalid_client("The client assertion is not a well-formed JWT"))?;

    let assertion_client_id = assertion_claims
        .get("sub")
        .or_else(|| assertion_claims.get("iss"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            OAuth2Error::invalid_client("The client assertion does not name the client")
        })?
        .to_string();

    if let Some(client_id) = client_id {
        if client_id != assertion_client_id {
            return Err(OAuth2Error::invalid_client(
                "The client assertion names another client",
            ));
        }
    }

    let registered_client = context
        .registered_clients()
        .find_by_client_id(&assertion_client_id)
        .ok_or_else(|| client_not_found(&assertion_client_id))?;

    if !registered_client
        .client_authentication_methods()
        .contains(&ClientAuthenticationMethod::PrivateKeyJwt)
    {
        return Err(OAuth2Error::invalid_client(
            "The client is not registered for the private_key_jwt authentication method",
        ));
    }

    let jwk_set = registered_client
        .client_settings()
        .jwk_set()
        .ok_or_else(|| {
            OAuth2Error::invalid_client("The client has no key to verify the assertion with")
        })?;

    let decoding_key = assertion_decoding_key(jwk_set, &header)?;
    let issuer = context.issuer();
    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[assertion_client_id.as_str()]);
    validation.set_audience(&[
        issuer.as_str(),
        format!(
            "{issuer}{}",
            context.settings().token_endpoint()
        )
        .as_str(),
    ]);
    validation.leeway = 60;

    let decoded = jsonwebtoken::decode::<Value>(assertion, &decoding_key, &validation)
        .map_err(|error| {
            OAuth2Error::invalid_client(format!("The client assertion is not valid: {error}"))
        })?;

    let subject = decoded.claims.get("sub").and_then(Value::as_str);
    if subject != Some(assertion_client_id.as_str()) {
        return Err(OAuth2Error::invalid_client(
            "The client assertion does not name the client consistently",
        ));
    }

    Ok(registered_client)
}

/// Builds the key that verifies a client assertion.
///
/// # Arguments
///
/// * `jwk_set` - The keys the client registered.
/// * `header` - The header of the assertion.
fn assertion_decoding_key(
    jwk_set: &JwkSet,
    header: &jsonwebtoken::Header,
) -> Result<DecodingKey, OAuth2Error> {
    let jwk = match header.kid.as_deref() {
        Some(kid) => jwk_set.find_by_kid(kid),
        None => jwk_set.first(),
    }
    .ok_or_else(|| OAuth2Error::invalid_client("The key of the client assertion is unknown"))?;

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
        return Err(OAuth2Error::invalid_client(
            "The algorithm of the client assertion does not match the key of the client",
        ));
    }

    match jwk.kty() {
        "oct" => jwk
            .secret()
            .map(|secret| DecodingKey::from_secret(&secret))
            .ok_or_else(|| {
                OAuth2Error::invalid_client("The key of the client assertion is invalid")
            }),
        "RSA" => {
            let (Some(modulus), Some(exponent)) = (jwk.modulus(), jwk.exponent()) else {
                return Err(OAuth2Error::invalid_client(
                    "The key of the client assertion is incomplete",
                ));
            };
            DecodingKey::from_rsa_components(modulus, exponent).map_err(|_| {
                OAuth2Error::invalid_client("The key of the client assertion is invalid")
            })
        }
        _ => Err(OAuth2Error::invalid_client(
            "The type of the key of the client assertion is not supported",
        )),
    }
}

/// Authenticates a client that proves its identity with a certificate.
///
/// # Arguments
///
/// * `client_id` - The identifier of the client.
/// * `subject` - The subject of the certificate of the connection.
/// * `context` - The context of the authorization server.
fn authenticate_tls_client(
    client_id: &str,
    subject: &str,
    context: &AuthorizationServerContext,
) -> Result<RegisteredClient, OAuth2Error> {
    let registered_client = context
        .registered_clients()
        .find_by_client_id(client_id)
        .ok_or_else(|| client_not_found(client_id))?;

    let method = registered_client
        .client_authentication_methods()
        .iter()
        .find(|method| {
            matches!(
                method,
                ClientAuthenticationMethod::TlsClientAuth
                    | ClientAuthenticationMethod::SelfSignedTlsClientAuth
            )
        })
        .copied()
        .ok_or_else(|| {
            OAuth2Error::invalid_client(
                "The client is not registered for a certificate authentication method",
            )
        })?;

    let registered_subject = registered_client
        .client_settings()
        .x509_certificate_subject_dn()
        .ok_or_else(|| {
            OAuth2Error::invalid_client("The client has no certificate subject registered")
        })?;

    if !registered_subject.eq_ignore_ascii_case(subject) {
        return Err(OAuth2Error::invalid_client(
            "The subject of the certificate is not the subject of the client",
        ));
    }

    tracing::debug!(
        client_id = client_id,
        method = method.value(),
        "The client authenticated with its certificate"
    );

    Ok(registered_client)
}

/// Returns the subject of the certificate of the connection, if the terminator
/// of the connection forwarded one.
///
/// The application reads the header of the terminator, because the connection
/// of the application is the connection the terminator opened.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn tls_client_certificate_subject(request: &dyn HttpRequest) -> Option<String> {
    [
        "x-client-cert-subject-dn",
        "x-ssl-client-s-dn",
        "ssl-client-s-dn",
    ]
    .into_iter()
    .find_map(|name| request.header(name))
    .map(|subject| subject.trim().to_string())
    .filter(|subject| !subject.is_empty())
}

/// Returns the credentials of the client of the request, if it presents any.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn client_credentials(
    request: &dyn HttpRequest,
) -> Option<(String, Option<String>, ClientAuthenticationMethod)> {
    if let Some((client_id, client_secret)) = basic_credentials(request) {
        return Some((
            client_id,
            Some(client_secret),
            ClientAuthenticationMethod::ClientSecretBasic,
        ));
    }

    let client_id = form_parameter(request, "client_id")?;
    let client_secret = form_parameter(request, "client_secret");
    let method = match client_secret {
        Some(_) => ClientAuthenticationMethod::ClientSecretPost,
        None => ClientAuthenticationMethod::None,
    };

    Some((client_id, client_secret, method))
}

/// Returns the credentials of the `Authorization` header of the request.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn basic_credentials(request: &dyn HttpRequest) -> Option<(String, String)> {
    let header = request.header("Authorization")?;
    let encoded = header.strip_prefix("Basic ")?;
    let decoded = STANDARD.decode(encoded.trim()).ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    let (client_id, client_secret) = decoded.split_once(':')?;

    Some((
        urlencoding::decode(client_id)
            .map(|value| value.into_owned())
            .unwrap_or_else(|_| client_id.to_string()),
        urlencoding::decode(client_secret)
            .map(|value| value.into_owned())
            .unwrap_or_else(|_| client_secret.to_string()),
    ))
}

/// Decodes a percent-encoded value.
///
/// # Arguments
///
/// * `value` - The value to decode.
fn decode(value: &str) -> String {
    urlencoding::decode(value)
        .map(|value| value.into_owned())
        .unwrap_or_else(|_| value.to_string())
}

/// Returns the claims of a JWT without verifying its signature.
///
/// The claims of a client assertion are read before the client they name is
/// known, because the key that verifies the assertion is the key of that
/// client. The signature is verified with the key of the client afterwards, so
/// the claims that were read first are not trusted on their own.
///
/// # Arguments
///
/// * `token` - The token to read.
fn unverified_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    let bytes = crate::oauth2::jose::base64_url_decode(payload)?;

    serde_json::from_slice(&bytes).ok()
}

/// Returns the value of a claim of a JSON document.
///
/// # Arguments
///
/// * `value` - The document.
/// * `name` - The name of the claim.
pub fn claim<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    value.get(name).and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use next_web_core::http::buffer_request_body;
    use next_web_core::http::HttpRequest;

    use super::{authenticate_client, form_parameter, form_parameter_map};
    use crate::oauth2::server::authorization::{
        client::{
            AuthorizationGrantType, ClientAuthenticationMethod, InMemoryRegisteredClientRepository,
            RegisteredClient,
        },
        context::AuthorizationServerContext,
    };

    fn context() -> AuthorizationServerContext {
        let confidential = RegisteredClient::with_client_id("demo-client")
            .set_client_secret("{noop}secret")
            .client_authentication_method(ClientAuthenticationMethod::ClientSecretBasic)
            .client_authentication_method(ClientAuthenticationMethod::ClientSecretPost)
            .authorization_grant_type(AuthorizationGrantType::AuthorizationCode);
        let public = RegisteredClient::with_client_id("public-client")
            .client_authentication_method(ClientAuthenticationMethod::None);

        AuthorizationServerContext::builder()
            .registered_clients(std::sync::Arc::new(
                InMemoryRegisteredClientRepository::new([confidential, public]),
            ))
            .build()
    }

    #[test]
    fn a_client_authenticates_with_the_authorization_header() {
        let credentials = STANDARD.encode("demo-client:secret");
        let request = Request::builder()
            .uri("/oauth2/token")
            .header("Authorization", format!("Basic {credentials}"))
            .body(Body::empty())
            .unwrap();

        let client = authenticate_client(&request, &context()).expect("the client authenticates");

        assert_eq!(client.client_id(), "demo-client");
    }

    #[test]
    fn a_client_authenticates_with_the_form_parameters() {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=demo-client&client_secret=secret");

        let client = authenticate_client(&request, &context()).expect("the client authenticates");

        assert_eq!(client.client_id(), "demo-client");
    }

    #[test]
    fn a_public_client_authenticates_with_its_identifier() {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=public-client");

        let client = authenticate_client(&request, &context()).expect("the client authenticates");

        assert_eq!(client.client_id(), "public-client");
    }

    #[test]
    fn an_unknown_client_is_reported() {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=other&client_secret=secret");

        let error = authenticate_client(&request, &context()).expect_err("the client is unknown");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn a_wrong_secret_is_reported() {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=demo-client&client_secret=wrong");

        let error = authenticate_client(&request, &context()).expect_err("the secret is wrong");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn a_request_without_credentials_is_reported() {
        let request = Request::builder()
            .uri("/oauth2/token")
            .body(Body::empty())
            .unwrap();

        let error = authenticate_client(&request, &context())
            .expect_err("client authentication is required");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn the_parameters_of_the_query_and_the_form_are_read() {
        let mut request = Request::builder()
            .uri("/oauth2/token?query=1")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "body=2");

        assert_eq!(form_parameter(&request, "query").as_deref(), Some("1"));
        assert_eq!(form_parameter(&request, "body").as_deref(), Some("2"));
        assert_eq!(
            form_parameter_map(&request),
            vec![("body".to_string(), "2".to_string())]
        );
    }

    /// The secret the assertion of the tests is signed with.
    const ASSERTION_SECRET: &str = "the-assertion-secret";

    /// Returns a request that presents a client assertion.
    fn assertion_request(assertion: &str) -> Request {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(
            &mut request,
            format!(
                "grant_type=client_credentials&client_assertion_type={}&client_assertion={assertion}",
                super::CLIENT_ASSERTION_TYPE_JWT_BEARER
            ),
        );
        request
    }

    /// Returns an assertion that names the given client.
    fn assertion(client_id: &str) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};

        let claims = serde_json::json!({
            "iss": client_id,
            "sub": client_id,
            "aud": "http://localhost",
            "jti": "the-token-id",
            "exp": 4_102_444_800_u64,
        });

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(ASSERTION_SECRET.as_bytes()),
        )
        .expect("the assertion is signed")
    }

    /// Returns a context with a client that authenticates with an assertion.
    fn assertion_context() -> AuthorizationServerContext {
        use crate::oauth2::jose::{Jwk, JwkSet};

        let client = RegisteredClient::with_client_id("assertion-client")
            .client_authentication_method(ClientAuthenticationMethod::PrivateKeyJwt)
            .set_client_settings(
                crate::oauth2::server::authorization::client::ClientSettings::builder()
                    .jwk_set(JwkSet::new(vec![Jwk::symmetric(ASSERTION_SECRET)]))
                    .build(),
            );

        AuthorizationServerContext::builder()
            .registered_clients(std::sync::Arc::new(
                InMemoryRegisteredClientRepository::new([client]),
            ))
            .build()
    }

    #[test]
    fn a_client_authenticates_with_a_signed_assertion() {
        let request = assertion_request(&assertion("assertion-client"));

        let client = authenticate_client(&request, &assertion_context())
            .expect("the assertion authenticates the client");

        assert_eq!(client.client_id(), "assertion-client");
    }

    #[test]
    fn an_assertion_of_another_client_is_rejected() {
        let request = assertion_request(&assertion("other-client"));

        let error = authenticate_client(&request, &assertion_context())
            .expect_err("the client is unknown");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn an_assertion_with_a_broken_signature_is_rejected() {
        let assertion = assertion("assertion-client");
        // Keep the header and the claims and replace the signature, which is the
        // classic attempt to change the claims of a signed token.
        let (head, _) = assertion
            .rsplit_once('.')
            .expect("the assertion has a signature");
        let tampered = format!("{head}.AAAA");
        let request = assertion_request(&tampered);

        let error = authenticate_client(&request, &assertion_context())
            .expect_err("the signature does not match");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn an_assertion_of_a_client_that_is_not_registered_for_it_is_rejected() {
        // The client only authenticates with its secret, so its assertion is
        // not accepted.
        let request = assertion_request(&assertion("demo-client"));

        let error = authenticate_client(&request, &context())
            .expect_err("the client is not registered for an assertion");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn a_client_authenticates_with_the_subject_of_its_certificate() {
        let context = tls_context(true);
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("x-client-cert-subject-dn", "CN=mtls-client")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=mtls-client");

        let client = authenticate_client(&request, &context)
            .expect("the certificate authenticates the client");

        assert_eq!(client.client_id(), "mtls-client");
    }

    #[test]
    fn a_certificate_of_another_subject_is_rejected() {
        let context = tls_context(true);
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("x-client-cert-subject-dn", "CN=other-client")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=mtls-client");

        let error = authenticate_client(&request, &context)
            .expect_err("the certificate is of another subject");

        assert_eq!(error.error(), "invalid_client");
    }

    #[test]
    fn a_forwarded_certificate_is_refused_when_it_is_not_trusted() {
        let mut request = Request::builder()
            .uri("/oauth2/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("x-client-cert-subject-dn", "CN=mtls-client")
            .body(Body::empty())
            .unwrap();
        buffer_request_body(&mut request, "client_id=mtls-client");

        // The certificate of the connection is not read while the application
        // does not trust the terminator that forwarded it, so the request is
        // not authenticated by the certificate.
        let error = authenticate_client(&request, &tls_context(false))
            .expect_err("the forwarded certificate is not trusted");

        assert_eq!(error.error(), "invalid_client");
    }

    /// Returns a context with a client that authenticates with the subject of
    /// its certificate.
    ///
    /// # Arguments
    ///
    /// * `trust_forwarded_client_certificate` - Whether the forwarded
    ///   certificate is trusted.
    fn tls_context(trust_forwarded_client_certificate: bool) -> AuthorizationServerContext {
        use crate::oauth2::server::authorization::{
            client::ClientSettings, settings::AuthorizationServerSettings,
        };

        let client = RegisteredClient::with_client_id("mtls-client")
            .client_authentication_method(ClientAuthenticationMethod::TlsClientAuth)
            .set_client_settings(
                ClientSettings::builder()
                    .x509_certificate_subject_dn("CN=mtls-client")
                    .build(),
            );

        AuthorizationServerContext::builder()
            .settings(
                AuthorizationServerSettings::builder()
                    .trust_forwarded_client_certificate(trust_forwarded_client_certificate)
                    .build(),
            )
            .registered_clients(std::sync::Arc::new(
                InMemoryRegisteredClientRepository::new([client]),
            ))
            .build()
    }
}
