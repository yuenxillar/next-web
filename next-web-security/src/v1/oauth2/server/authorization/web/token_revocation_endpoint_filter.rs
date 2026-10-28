//! The endpoint that revokes a token.

use next_web_core::{
    async_trait,
    http::filter::FilterError,
    http::{HttpMethod, StatusCode},
    http::filter::{HttpFilter, HttpFilterChain},
    http::{HttpRequest, HttpResponse},
    Named,
};
use serde_json::json;

use super::{
    super::{
        client_authentication::{authenticate_client, form_parameter},
        context::AuthorizationServerContext,
        error::OAuth2Error,
    },
    support::{error_response, json_response},
};

/// Serves the token revocation endpoint of the authorization server.
///
/// Revoking the refresh token of an authorization revokes the authorization,
/// and revoking its access token revokes the access token. Only the tokens of
/// the authenticated client can be revoked.
#[derive(Clone)]
pub struct TokenRevocationEndpointFilter {
    context: AuthorizationServerContext,
}

impl TokenRevocationEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }
}

#[async_trait]
impl HttpFilter for TokenRevocationEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().revocation_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request("The revocation endpoint supports POST"),
            );
            return Ok(());
        }

        let registered_client = match authenticate_client(request, &self.context) {
            Ok(registered_client) => registered_client,
            Err(error) => {
                error_response(response, &error);
                return Ok(());
            }
        };

        let Some(token) = form_parameter(request, "token") else {
            error_response(
                response,
                &OAuth2Error::invalid_request("The token parameter is required"),
            );
            return Ok(());
        };

        // Revoking a token that is unknown or that belongs to another client
        // succeeds, which is what the specification asks for: the endpoint must
        // not report whether a token exists.
        if let Some(authorization) = self
            .context
            .authorizations()
            .find_by_token(&token, None)
            .filter(|authorization| {
                authorization.registered_client_id() == registered_client.id_value()
            })
        {
            let is_refresh_token = authorization
                .refresh_token_value()
                .is_some_and(|refresh_token| refresh_token.token_value() == token);

            if is_refresh_token {
                self.context.authorizations().remove(&authorization);
            } else {
                let mut authorization = authorization;
                authorization.remove_access_token();
                self.context.authorizations().save(authorization);
            }

            tracing::debug!(
                "The {token} token of the {} client is revoked",
                registered_client.client_id()
            );
        }

        json_response(response, StatusCode::OK, &json!({}));
        Ok(())
    }
}

impl Named for TokenRevocationEndpointFilter {
    fn name(&self) -> &str {
        "TokenRevocationEndpointFilter"
    }
}
