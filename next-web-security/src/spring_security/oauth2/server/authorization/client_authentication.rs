//! Authenticates the client of a request that reaches a protected endpoint.

use base64::{engine::general_purpose::STANDARD, Engine};
use next_web_core::http::form_parameters;
use next_web_core::traits::http::http_request::HttpRequest;
use serde_json::Value;

use super::{
    client::{
        client_not_found, client_secret_matches, ClientAuthenticationMethod, RegisteredClient,
    },
    context::AuthorizationServerContext,
    error::OAuth2Error,
};

/// Returns the value of a form parameter of the request.
///
/// The query of the request is looked up first, and the url-encoded body of the
/// request is looked up next. The body is only available when the web layer
/// buffered it, which it does for a form post.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
/// * `name` - The name of the parameter.
pub fn form_parameter(request: &dyn HttpRequest, name: &str) -> Option<String> {
    if let Some(value) = request.parameter(name) {
        return Some(decode(value));
    }

    let body = request.body_bytes()?;
    form_parameters(body)
        .into_iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
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
    use next_web_core::traits::http::http_request::HttpRequest;

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
}
