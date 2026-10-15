//! The endpoints of the authorization server.

mod authorization_endpoint_filter;
mod client_registration_endpoint_filter;
mod device_authorization_endpoint_filter;
mod device_verification_endpoint_filter;
mod jwk_set_endpoint_filter;
mod logout_endpoint_filter;
mod metadata_endpoint_filter;
mod pushed_authorization_request_endpoint_filter;
pub(crate) mod support;
mod token_endpoint_filter;
mod token_introspection_endpoint_filter;
mod token_revocation_endpoint_filter;
mod user_info_endpoint_filter;

pub use authorization_endpoint_filter::AuthorizationEndpointFilter;
pub use client_registration_endpoint_filter::OAuth2ClientRegistrationEndpointFilter;
pub use device_authorization_endpoint_filter::OAuth2DeviceAuthorizationEndpointFilter;
pub use device_verification_endpoint_filter::OAuth2DeviceVerificationEndpointFilter;
pub use jwk_set_endpoint_filter::JwkSetEndpointFilter;
pub use logout_endpoint_filter::LogoutEndpointFilter;
pub use metadata_endpoint_filter::AuthorizationServerMetadataEndpointFilter;
pub use pushed_authorization_request_endpoint_filter::OAuth2PushedAuthorizationRequestEndpointFilter;
pub use token_endpoint_filter::TokenEndpointFilter;
pub use token_introspection_endpoint_filter::TokenIntrospectionEndpointFilter;
pub use token_revocation_endpoint_filter::TokenRevocationEndpointFilter;
pub use user_info_endpoint_filter::UserInfoEndpointFilter;

/// The helpers the tests of the endpoints share.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
    use next_web_core::{
        filter::application_filter_chain::ApplicationFilterChain,
        http::{buffer_request_body, form_parameters},
        traits::{
            filter::{HttpFilter, HttpFilterChain},
            http::{HttpRequest, HttpResponse},
        },
    };
    use serde_json::Value;

    /// Runs one request through the given filters.
    ///
    /// # Arguments
    ///
    /// * `filters` - The filters to run.
    /// * `request` - The request to run them against.
    pub(crate) async fn run(filters: &[Arc<dyn HttpFilter>], request: Request) -> Response {
        let chain = ApplicationFilterChain::new(Arc::from(filters.to_vec()));
        let mut request = request;
        request.ready();
        let mut response = Response::new(Body::empty());

        chain
            .do_filter(&mut request, &mut response)
            .await
            .expect("the filters handle the request");

        response
    }

    /// Builds a GET request for a path with a query.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    /// * `query` - The parameters of the request.
    pub(crate) fn get_request(path: &str, query: &[(&str, &str)]) -> Request {
        Request::builder()
            .uri(uri(path, query))
            .body(Body::empty())
            .expect("the request is built")
    }

    /// Builds a POST request with a url-encoded form body.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    /// * `form` - The parameters of the form.
    pub(crate) fn form_request(path: &str, form: &[(&str, &str)]) -> Request {
        let mut request = Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(Body::empty())
            .expect("the request is built");
        buffer_request_body(&mut request, encode_form(form));
        request
    }

    /// Builds a POST request with a url-encoded form body and a client that
    /// authenticates with the `Authorization` header.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the request.
    /// * `form` - The parameters of the form.
    /// * `credentials` - The client id and secret of the client.
    pub(crate) fn form_request_with_basic_client(
        path: &str,
        form: &[(&str, &str)],
        credentials: (&str, &str),
    ) -> Request {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let encoded = STANDARD.encode(format!("{}:{}", credentials.0, credentials.1));
        Request::builder()
            .method("POST")
            .uri(path)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Authorization", format!("Basic {encoded}"))
            .body(Body::from(encode_form(form)))
            .expect("the request is built")
    }

    /// Returns the JSON body of a response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    pub(crate) async fn json_body(response: Response) -> Value {
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("the body of the response is read");
        serde_json::from_slice(&bytes).expect("the body of the response is JSON")
    }

    /// Returns the location of a redirect response.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    pub(crate) fn location(response: &Response) -> Option<String> {
        response
            .headers()
            .get(next_web_core::http::header::LOCATION)
            .and_then(|location| location.to_str().ok())
            .map(ToString::to_string)
    }

    /// Returns whether the response is committed.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    pub(crate) fn is_committed(response: &Response) -> bool {
        HttpResponse::is_committed(response)
    }

    /// Returns the status of the response, which is `200` when the response is
    /// not committed yet.
    ///
    /// # Arguments
    ///
    /// * `response` - The response of the request.
    pub(crate) fn status(response: &Response) -> StatusCode {
        response.status()
    }

    /// Builds the URI of a request.
    fn uri(path: &str, query: &[(&str, &str)]) -> String {
        if query.is_empty() {
            return path.to_string();
        }
        format!("{path}?{}", encode_form(query))
    }

    /// Encodes the parameters of a form.
    fn encode_form(form: &[(&str, &str)]) -> String {
        form.iter()
            .map(|(name, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(name),
                    urlencoding::encode(value)
                )
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Returns the parameters of the body of a request, which keeps the helper
    /// used by the tests of the form parsing.
    #[allow(dead_code)]
    pub(crate) fn body_parameters(request: &dyn HttpRequest) -> Vec<(String, String)> {
        request
            .body_bytes()
            .map(|body| {
                form_parameters(body)
                    .into_iter()
                    .map(|(name, value)| (name.into_owned(), value.into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
