//! The endpoint that publishes the keys of the authorization server.

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

use super::{super::context::AuthorizationServerContext, support::json_response};

/// Serves the JWK set of the authorization server.
///
/// The set holds the keys the access tokens of the server are signed with, so a
/// resource server can verify the tokens it receives.
#[derive(Clone)]
pub struct JwkSetEndpointFilter {
    context: AuthorizationServerContext,
}

impl JwkSetEndpointFilter {
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
impl HttpFilter for JwkSetEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().jwk_set_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != HttpMethod::GET {
            response.set_status_code(StatusCode::METHOD_NOT_ALLOWED);
            response.commit();
            return Ok(());
        }

        let jwk_source = self.context.jwk_source();
        json_response(response, StatusCode::OK, &jwk_source.jwk_set().to_json());

        Ok(())
    }
}

impl Named for JwkSetEndpointFilter {
    fn name(&self) -> &str {
        "JwkSetEndpointFilter"
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use next_web_core::{http::StatusCode, traits::filter::HttpFilter};

    use super::{super::test_support, JwkSetEndpointFilter};
    use crate::oauth2::{
        jose::{JwtSigner, SecretJwtSigner},
        server::authorization::{
            client::InMemoryRegisteredClientRepository, context::AuthorizationServerContext,
        },
    };

    #[tokio::test]
    async fn the_endpoint_publishes_the_keys_of_the_server() {
        let context = AuthorizationServerContext::builder()
            .registered_clients(Arc::new(InMemoryRegisteredClientRepository::default()))
            .jwt_signer(Arc::new(SecretJwtSigner::new("secret")) as Arc<dyn JwtSigner>)
            .build();
        let filters: Arc<[Arc<dyn HttpFilter>]> =
            Arc::from(vec![
                Arc::new(JwkSetEndpointFilter::new(context)) as Arc<dyn HttpFilter>
            ]);

        let response =
            test_support::run(&filters, test_support::get_request("/oauth2/jwks", &[])).await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = test_support::json_body(response).await;
        assert_eq!(body["keys"][0]["kty"], "oct");
        assert_eq!(body["keys"][0]["alg"], "HS256");
    }

    #[tokio::test]
    async fn another_path_continues_the_chain() {
        let context = AuthorizationServerContext::builder()
            .registered_clients(Arc::new(InMemoryRegisteredClientRepository::default()))
            .build();
        let filters: Arc<[Arc<dyn HttpFilter>]> =
            Arc::from(vec![
                Arc::new(JwkSetEndpointFilter::new(context)) as Arc<dyn HttpFilter>
            ]);

        let response = test_support::run(&filters, test_support::get_request("/other", &[])).await;

        assert!(!test_support::is_committed(&response));
    }
}
