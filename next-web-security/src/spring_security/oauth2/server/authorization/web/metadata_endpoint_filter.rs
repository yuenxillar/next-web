//! The endpoint that publishes the metadata of the authorization server.

use next_web_core::{
    async_trait,
    http::StatusCode,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use super::{super::context::AuthorizationServerContext, support::json_response};

/// Serves the metadata document of the authorization server.
///
/// The document is the `oauth-authorization-server` metadata of the
/// specification, which is what a client reads to discover the endpoints and
/// the capabilities of the server.
#[derive(Clone)]
pub struct AuthorizationServerMetadataEndpointFilter {
    context: AuthorizationServerContext,
}

impl AuthorizationServerMetadataEndpointFilter {
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
impl HttpFilter for AuthorizationServerMetadataEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), next_web_core::filter::FilterError> {
        let settings = self.context.settings();
        if request.path() != settings.authorization_server_metadata_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        if request.method() != next_web_core::http::HttpMethod::GET {
            response.set_status_code(StatusCode::METHOD_NOT_ALLOWED);
            response.commit();
            return Ok(());
        }

        json_response(response, StatusCode::OK, &settings.metadata());
        Ok(())
    }
}

impl Named for AuthorizationServerMetadataEndpointFilter {
    fn name(&self) -> &str {
        "AuthorizationServerMetadataEndpointFilter"
    }
}
