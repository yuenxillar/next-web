//! The endpoint with which a client registers itself and manages its
//! registration.

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{HttpRequest, HttpResponse},
        named::Named,
    },
};
use serde_json::{json, Value};

use super::support::{error_response, json_response};
use crate::oauth2::server::authorization::{
    client::{
        AuthorizationGrantType, ClientAuthenticationMethod, ClientSettings, RegisteredClient,
        TokenSettings,
    },
    context::AuthorizationServerContext,
    error::OAuth2Error,
    token::now,
};

/// Serves the client registration endpoint of the authorization server.
///
/// A client registers the metadata of its own registration and receives the
/// identifier and the secret it authenticates with, reads its registration
/// back, replaces it, or removes it. The endpoint is the one of the OpenID
/// Connect dynamic client registration, so the metadata it answers with can be
/// given back to the endpoint.
///
/// The endpoint is not open: a request is served only when the application
/// enabled open registration, or when the request presents the initial access
/// token of the endpoint as a bearer token. An endpoint that any client may
/// register itself with is an endpoint any client may use to obtain
/// credentials, so open registration has to be turned on deliberately.
#[derive(Clone)]
pub struct OAuth2ClientRegistrationEndpointFilter {
    context: AuthorizationServerContext,
}

impl OAuth2ClientRegistrationEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }

    /// Returns whether the request may use the endpoint.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    fn authorize(&self, request: &dyn HttpRequest) -> Result<(), OAuth2Error> {
        if self.context.settings().allow_open_client_registration() {
            return Ok(());
        }

        let Some(initial_access_token) = self
            .context
            .settings()
            .client_registration_initial_access_token()
        else {
            return Err(OAuth2Error::invalid_token(
                "The client registration endpoint is not enabled",
            ));
        };

        let presented = request
            .header("Authorization")
            .and_then(|header| header.strip_prefix("Bearer "))
            .map(str::trim)
            .unwrap_or_default();
        if !secret_matches(presented, initial_access_token) {
            return Err(OAuth2Error::invalid_token(
                "The initial access token of the client registration endpoint is not valid",
            ));
        }

        Ok(())
    }

    /// Serves a request to the endpoint.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `client_id` - The client the request names, if it names one.
    fn handle(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        client_id: Option<&str>,
    ) -> Result<(), FilterError> {
        if let Err(error) = self.authorize(request) {
            error_response(response, &error);
            return Ok(());
        }

        let result = match (request.method(), client_id) {
            (HttpMethod::POST, None) => self.register(request, response),
            (HttpMethod::GET, Some(client_id)) => self.read(response, client_id),
            (HttpMethod::PUT, Some(client_id)) => self.update(request, response, client_id),
            (HttpMethod::DELETE, Some(client_id)) => self.remove(response, client_id),
            _ => {
                error_response(
                    response,
                    &OAuth2Error::invalid_request(
                        "The client registration endpoint supports POST to register, and GET, \
                         PUT, and DELETE of a registered client",
                    ),
                );
                return Ok(());
            }
        };

        if let Err(error) = result {
            error_response(response, &error);
        }

        Ok(())
    }

    /// Registers the client of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn register(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), OAuth2Error> {
        let metadata = request_metadata(request)?;
        let client_id = next_web_core::Uuid::new_v4().to_string();
        let client_secret = next_web_core::Uuid::new_v4().simple().to_string();
        let registered_client =
            registered_client_of(&metadata, client_id.clone(), Some(client_secret.clone()))?;

        self.context
            .registered_clients()
            .save(registered_client.clone());

        let mut body = client_metadata(&registered_client);
        body["client_secret"] = json!(client_secret);
        body["client_secret_expires_at"] = json!(0);
        json_response(response, StatusCode::CREATED, &body);

        Ok(())
    }

    /// Answers with the registration of a client.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the endpoint.
    /// * `client_id` - The client of the request.
    fn read(&self, response: &mut dyn HttpResponse, client_id: &str) -> Result<(), OAuth2Error> {
        let Some(registered_client) = self.find(client_id) else {
            not_found_response(response, client_id);
            return Ok(());
        };

        json_response(
            response,
            StatusCode::OK,
            &client_metadata(&registered_client),
        );

        Ok(())
    }

    /// Replaces the registration of a client.
    ///
    /// The secret of the client is kept, because the metadata of a client
    /// registration does not carry it.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `client_id` - The client of the request.
    fn update(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        client_id: &str,
    ) -> Result<(), OAuth2Error> {
        let Some(existing) = self.find(client_id) else {
            not_found_response(response, client_id);
            return Ok(());
        };
        let metadata = request_metadata(request)?;
        let registered_client = registered_client_of(
            &metadata,
            client_id.to_string(),
            existing.client_secret().map(ToString::to_string),
        )?;

        self.context
            .registered_clients()
            .save(registered_client.clone());
        json_response(
            response,
            StatusCode::OK,
            &client_metadata(&registered_client),
        );

        Ok(())
    }

    /// Removes the registration of a client.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the endpoint.
    /// * `client_id` - The client of the request.
    fn remove(&self, response: &mut dyn HttpResponse, client_id: &str) -> Result<(), OAuth2Error> {
        if !self.context.registered_clients().remove(client_id) {
            not_found_response(response, client_id);
            return Ok(());
        }

        response.set_status_code(StatusCode::NO_CONTENT);
        response.set_body(Vec::new());

        Ok(())
    }

    /// Returns the registration of a client, if one is stored.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The client of the request.
    fn find(&self, client_id: &str) -> Option<RegisteredClient> {
        self.context
            .registered_clients()
            .find_by_client_id(client_id)
    }
}

#[async_trait]
impl HttpFilter for OAuth2ClientRegistrationEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let endpoint = self.context.settings().client_registration_endpoint();
        let path = request.path();

        if path == endpoint {
            return self.handle(request, response, None);
        }

        if let Some(client_id) = path
            .strip_prefix(endpoint)
            .and_then(|rest| rest.strip_prefix('/'))
            .filter(|client_id| !client_id.is_empty() && !client_id.contains('/'))
        {
            return self.handle(request, response, Some(client_id));
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for OAuth2ClientRegistrationEndpointFilter {
    fn name(&self) -> &str {
        "OAuth2ClientRegistrationEndpointFilter"
    }
}

/// Reads the metadata of a client registration request.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn request_metadata(request: &dyn HttpRequest) -> Result<Value, OAuth2Error> {
    let Some(body) = request.body_bytes() else {
        return Err(OAuth2Error::invalid_client_metadata(
            "The metadata of the client is required",
        ));
    };

    serde_json::from_slice(body).map_err(|error| {
        OAuth2Error::invalid_client_metadata(format!(
            "The metadata of the client is not a valid document: {error}"
        ))
    })
}

/// Builds the registration of a client from its metadata.
///
/// # Arguments
///
/// * `metadata` - The metadata of the request.
/// * `client_id` - The identifier of the client.
/// * `client_secret` - The secret of the client, if the client has one.
fn registered_client_of(
    metadata: &Value,
    client_id: String,
    client_secret: Option<String>,
) -> Result<RegisteredClient, OAuth2Error> {
    let method = metadata
        .get("token_endpoint_auth_method")
        .and_then(Value::as_str)
        .unwrap_or("client_secret_basic");
    let authentication_method =
        ClientAuthenticationMethod::from_value(method).ok_or_else(|| {
            OAuth2Error::invalid_client_metadata(format!(
                "The {method} authentication method is not supported"
            ))
        })?;

    let grant_types = metadata
        .get("grant_types")
        .and_then(Value::as_array)
        .map(|grant_types| {
            grant_types
                .iter()
                .filter_map(Value::as_str)
                .filter_map(AuthorizationGrantType::from_value)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![AuthorizationGrantType::AuthorizationCode]);
    if grant_types.contains(&AuthorizationGrantType::AuthorizationCode)
        && metadata
            .get("redirect_uris")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    {
        return Err(OAuth2Error::invalid_client_metadata(
            "A client that uses the authorization code grant needs a redirect URI",
        ));
    }

    let client_name = metadata
        .get("client_name")
        .and_then(Value::as_str)
        .unwrap_or("Registered Client");

    let mut registered_client = RegisteredClient::with_client_id(client_id.clone())
        .set_id(client_id)
        .set_client_name(client_name)
        .client_authentication_method(authentication_method)
        .set_client_settings(
            ClientSettings::builder()
                .require_proof_key(false)
                .require_authorization_consent(true)
                .build(),
        )
        .set_token_settings(TokenSettings::default());
    if let Some(client_secret) = client_secret {
        registered_client = registered_client.set_client_secret(format!("{{noop}}{client_secret}"));
    }
    for grant_type in &grant_types {
        registered_client = registered_client.authorization_grant_type(*grant_type);
    }
    if let Some(redirect_uris) = metadata.get("redirect_uris").and_then(Value::as_array) {
        for redirect_uri in redirect_uris.iter().filter_map(Value::as_str) {
            registered_client = registered_client.redirect_uri(redirect_uri);
        }
    }
    if let Some(scopes) = metadata.get("scope").and_then(Value::as_str) {
        for scope in scopes.split_whitespace() {
            registered_client = registered_client.scope(scope);
        }
    }

    Ok(registered_client)
}

/// Builds the metadata document of a registration.
///
/// # Arguments
///
/// * `registered_client` - The registration.
fn client_metadata(registered_client: &RegisteredClient) -> Value {
    let mut metadata = json!({
        "client_id": registered_client.client_id(),
        "client_id_issued_at": now(),
        "client_name": registered_client.client_name(),
        "grant_types": registered_client
            .authorization_grant_types()
            .iter()
            .map(|grant_type| grant_type.value())
            .collect::<Vec<_>>(),
        "redirect_uris": registered_client.redirect_uris(),
        "scope": registered_client.scopes().join(" "),
        "token_endpoint_auth_method": registered_client
            .client_authentication_methods()
            .first()
            .map(ClientAuthenticationMethod::value),
    });

    // The secret of a registration is reported as it was issued. A secret the
    // application stored as a hash cannot be reported, because the value that
    // was issued is not known any more.
    if let Some(client_secret) = registered_client.client_secret().and_then(plain_secret) {
        metadata["client_secret"] = json!(client_secret);
        metadata["client_secret_expires_at"] = json!(0);
    }

    metadata
}

/// Returns the secret a client presents, when the stored value is one.
///
/// A stored value that carries an encoding prefix such as `{noop}` is the
/// secret itself, and a value that is a hash of the secret is not known.
///
/// # Arguments
///
/// * `stored` - The secret of the registration.
fn plain_secret(stored: &str) -> Option<&str> {
    match stored.strip_prefix('{') {
        Some(rest) => rest.split_once('}').map(|(_, secret)| secret),
        None => Some(stored),
    }
}

/// Writes the response of a request that names an unknown client.
///
/// # Arguments
///
/// * `response` - The response of the endpoint.
/// * `client_id` - The client of the request.
fn not_found_response(response: &mut dyn HttpResponse, client_id: &str) {
    json_response(
        response,
        StatusCode::NOT_FOUND,
        &json!({
            "error": "not_found",
            "error_description": format!("The registered client {client_id} is unknown"),
        }),
    );
}

/// Compares two values without stopping at the first character that differs.
///
/// # Arguments
///
/// * `presented` - The value of the request.
/// * `expected` - The value of the registration.
fn secret_matches(presented: &str, expected: &str) -> bool {
    let presented = presented.as_bytes();
    let expected = expected.as_bytes();
    if presented.len() != expected.len() {
        return false;
    }

    presented
        .iter()
        .zip(expected)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}
