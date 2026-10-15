//! The endpoint that accepts an authorization request a client pushed.

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
use serde_json::json;

use super::support::{error_response, json_response};
use crate::oauth2::server::authorization::{
    client_authentication::{authenticate_client, form_parameter, request_parameters},
    context::AuthorizationServerContext,
    error::OAuth2Error,
    pushed_authorization_request::PushedAuthorizationRequest,
    token::now,
};

/// The prefix of the identifier of a pushed authorization request.
pub const REQUEST_URI_PREFIX: &str = "urn:ietf:params:oauth:request_uri:";

/// The lifetime of a pushed authorization request, in seconds.
pub const PUSHED_AUTHORIZATION_REQUEST_TIME_TO_LIVE: u64 = 90;

/// Serves the pushed authorization request endpoint of the authorization
/// server.
///
/// The endpoint accepts the parameters of an authorization request and answers
/// with a request URI. The client sends the request URI to the authorization
/// endpoint instead of the parameters, which is what keeps the parameters of a
/// large request out of the URL of the browser.
#[derive(Clone)]
pub struct OAuth2PushedAuthorizationRequestEndpointFilter {
    context: AuthorizationServerContext,
}

impl OAuth2PushedAuthorizationRequestEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }

    /// Stores the parameters of the request and answers with a request URI.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn push(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request(
                    "The pushed authorization request endpoint supports POST",
                ),
            );
            return Ok(());
        }

        let result = self.push_request(request, response);
        if let Err(error) = result {
            error_response(response, &error);
        }

        Ok(())
    }

    /// Stores the parameters of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn push_request(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), OAuth2Error> {
        // The client authenticates, because a pushed authorization request is
        // the first half of the authorization code grant of a client that has
        // credentials.
        let registered_client = authenticate_client(request, &self.context)?;

        let response_type =
            form_parameter(request, "response_type").unwrap_or_else(|| "code".to_string());
        if response_type != "code" {
            return Err(OAuth2Error::unsupported_response_type(
                "The authorization endpoint supports the code response type",
            ));
        }

        let parameters = request_parameters(request)
            .into_iter()
            .filter(|(name, _)| {
                !matches!(
                    name.as_str(),
                    "client_secret" | "client_assertion" | "client_assertion_type"
                )
            })
            .map(|(name, value)| {
                if name == "client_id" {
                    (name, registered_client.client_id().to_string())
                } else {
                    (name, value)
                }
            })
            .collect::<Vec<_>>();

        let request_uri = format!("{REQUEST_URI_PREFIX}{}", next_web_core::Uuid::new_v4());
        let expires_at = now() + PUSHED_AUTHORIZATION_REQUEST_TIME_TO_LIVE;
        self.context
            .pushed_authorization_requests()
            .save(PushedAuthorizationRequest::new(
                request_uri.clone(),
                registered_client.id_value(),
                parameters,
                expires_at,
            ));

        json_response(
            response,
            StatusCode::CREATED,
            &json!({
                "request_uri": request_uri,
                "expires_in": PUSHED_AUTHORIZATION_REQUEST_TIME_TO_LIVE,
            }),
        );

        Ok(())
    }
}

#[async_trait]
impl HttpFilter for OAuth2PushedAuthorizationRequestEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path()
            != self
                .context
                .settings()
                .pushed_authorization_request_endpoint()
        {
            return filter_chain.do_filter(request, response).await;
        }

        self.push(request, response)
    }
}

impl Named for OAuth2PushedAuthorizationRequestEndpointFilter {
    fn name(&self) -> &str {
        "OAuth2PushedAuthorizationRequestEndpointFilter"
    }
}
