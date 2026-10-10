//! The endpoint that publishes the information of the resource owner.

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};
use serde_json::json;

use super::{
    super::{
        authorization::OAuth2Authorization, context::AuthorizationServerContext,
        token::OAuth2TokenType,
    },
    support::{error_response, json_response},
};
use crate::oauth2::server::authorization::error::OAuth2Error;

/// Serves the user info endpoint of the authorization server.
///
/// The endpoint is asked by a client with the access token of its grant, and it
/// answers with the claims of the resource owner the grant was issued for. The
/// token is looked up in the authorizations of the server, so a client never
/// sees the claims of a token the server did not issue.
#[derive(Clone)]
pub struct UserInfoEndpointFilter {
    context: AuthorizationServerContext,
}

impl UserInfoEndpointFilter {
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
impl HttpFilter for UserInfoEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().user_info_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != HttpMethod::GET && request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request("The user info endpoint supports GET and POST"),
            );
            return Ok(());
        }

        let Some(access_token) = bearer_token(request) else {
            error_response(
                response,
                &OAuth2Error::invalid_token("The access token is required"),
            );
            return Ok(());
        };

        let authorization = self
            .context
            .authorizations()
            .find_by_token(&access_token, Some(OAuth2TokenType::AccessToken))
            .filter(|authorization| {
                authorization.is_access_token_active(super::super::token::now())
            });

        match authorization {
            Some(authorization) => {
                json_response(response, StatusCode::OK, &claims_of(&authorization))
            }
            None => error_response(
                response,
                &OAuth2Error::invalid_token("The access token is not active"),
            ),
        }

        Ok(())
    }
}

impl Named for UserInfoEndpointFilter {
    fn name(&self) -> &str {
        "UserInfoEndpointFilter"
    }
}

/// Returns the claims the user info endpoint answers with.
///
/// # Arguments
///
/// * `authorization` - The authorization of the access token.
fn claims_of(authorization: &OAuth2Authorization) -> serde_json::Value {
    json!({
        "sub": authorization.principal_name(),
        "name": authorization.principal_name(),
        "scope": authorization.scopes().join(" "),
    })
}

/// Returns the bearer token of the request, if it presents one.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn bearer_token(request: &dyn HttpRequest) -> Option<String> {
    request
        .header("Authorization")
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty())
        .or_else(|| super::super::client_authentication::form_parameter(request, "access_token"))
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};

    use super::{bearer_token, claims_of};
    use crate::oauth2::server::authorization::{
        authorization::OAuth2Authorization, token::OAuth2AccessToken,
    };

    #[test]
    fn the_claims_of_an_authorization_name_its_resource_owner() {
        let authorization = OAuth2Authorization::new("id", "demo-client", "user")
            .set_attribute("scopes", "openid read")
            .access_token(OAuth2AccessToken::new("token", 0, u64::MAX, vec![]));

        let claims = claims_of(&authorization);

        assert_eq!(claims["sub"], "user");
        assert_eq!(claims["name"], "user");
        assert_eq!(claims["scope"], "openid read");
    }

    #[test]
    fn the_access_token_of_a_request_is_read_from_the_header_or_the_form() {
        let header = Request::builder()
            .uri("/userinfo")
            .header("Authorization", "Bearer the-token")
            .body(Body::empty())
            .unwrap();
        assert_eq!(bearer_token(&header).as_deref(), Some("the-token"));

        let mut form = Request::builder()
            .uri("/userinfo")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .unwrap();
        next_web_core::http::buffer_request_body(&mut form, "access_token=the-token");
        assert_eq!(bearer_token(&form).as_deref(), Some("the-token"));
    }
}
