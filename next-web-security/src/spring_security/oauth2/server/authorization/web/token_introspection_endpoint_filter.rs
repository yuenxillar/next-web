//! The endpoint that reports the state of a token.

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

use super::{
    super::{
        authorization::OAuth2Authorization,
        client_authentication::{authenticate_client, form_parameter},
        context::AuthorizationServerContext,
        error::OAuth2Error,
        token::now,
    },
    support::{error_response, json_response},
};

/// Serves the token introspection endpoint of the authorization server.
///
/// The endpoint reports the state of the tokens it issued, which is what a
/// resource server that does not verify a token by itself reads. Only the state
/// of the tokens of the authenticated client is reported.
#[derive(Clone)]
pub struct TokenIntrospectionEndpointFilter {
    context: AuthorizationServerContext,
}

impl TokenIntrospectionEndpointFilter {
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
impl HttpFilter for TokenIntrospectionEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().introspection_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request("The introspection endpoint supports POST"),
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

        let introspection = self
            .context
            .authorizations()
            .find_by_token(&token, None)
            .filter(|authorization| {
                authorization.registered_client_id() == registered_client.id_value()
            })
            .filter(|authorization| {
                authorization.is_access_token_active(now())
                    || authorization.is_refresh_token_active(now())
            })
            .map(|authorization| describe(&authorization, &token))
            .unwrap_or_else(|| json!({ "active": false }));

        json_response(response, StatusCode::OK, &introspection);
        Ok(())
    }
}

impl Named for TokenIntrospectionEndpointFilter {
    fn name(&self) -> &str {
        "TokenIntrospectionEndpointFilter"
    }
}

/// Returns the introspection document of an authorization.
///
/// # Arguments
///
/// * `authorization` - The authorization the token belongs to.
/// * `token` - The token that was introspected.
fn describe(authorization: &OAuth2Authorization, token: &str) -> serde_json::Value {
    let mut document = json!({ "active": true });

    if let Some(access_token) = authorization.access_token_value() {
        if access_token.token_value() == token {
            document["token_type"] = json!("Bearer");
            document["exp"] = json!(access_token.expires_at());
            document["iat"] = json!(access_token.issued_at());
            document["client_id"] = json!(authorization.registered_client_id());
            document["scope"] = json!(access_token.scopes().join(" "));
            document["sub"] = json!(authorization.principal_name());
        }
    }

    if let Some(refresh_token) = authorization.refresh_token_value() {
        if refresh_token.token_value() == token {
            document["token_type"] = json!("refresh_token");
            document["exp"] = json!(refresh_token.expires_at());
            document["iat"] = json!(refresh_token.issued_at());
            document["client_id"] = json!(authorization.registered_client_id());
            document["scope"] = json!(authorization.scopes().join(" "));
            document["sub"] = json!(authorization.principal_name());
        }
    }

    document
}
