//! Tests of the resource server configuration.
//!
//! The configuration of a resource server is only usable when a request really
//! authenticates with the bearer token it carries, so the tests build a chain and
//! run a request through it the way the chain proxy does.

use std::{collections::HashMap, sync::Arc};

use axum::{body::Body, extract::Request};
use next_web_core::{async_trait, http::HttpRequest as _};

use super::*;
use crate::{
    config::{
        authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
        security_builder::SecurityBuilder,
        web::builders::HttpSecurity,
    },
    core::AuthenticationError,
    oauth2::server::resource::jwt::{Jwt, JwtDecoder},
    web::{security_filter_chain::SecurityFilterChain, test_support::Chain},
};

/// A decoder that accepts one token and refuses every other, so that the chain
/// can be asked whether it decodes and authenticates at all.
struct TestDecoder;

#[async_trait]
impl JwtDecoder for TestDecoder {
    async fn decode(&self, token: &str) -> Result<Jwt, AuthenticationError> {
        if token != "good-token" {
            return Err(AuthenticationError::new(
                "the token is not one this decoder accepts",
            ));
        }

        Ok(Jwt::new(token).with_claims(HashMap::from([
            ("sub".to_string(), "Alice".to_string()),
            ("scope".to_string(), "read write".to_string()),
        ])))
    }
}

/// Returns an HTTP security a configurer can be applied to.
fn http_security() -> HttpSecurity {
    HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())
}

/// Returns a `GET` of the application root, with the given authorization.
fn request(authorization: Option<&str>) -> Request {
    let mut builder = Request::builder().uri("https://sp.example.org/");
    if let Some(authorization) = authorization {
        builder = builder.header("authorization", authorization);
    }

    builder.body(Body::empty()).expect("a valid request")
}

/// Returns the names of the filters of a chain.
fn filter_names(mut http: HttpSecurity) -> Vec<String> {
    http.build()
        .get_filters()
        .iter()
        .map(|filter| filter.name().to_owned())
        .collect()
}

/// Returns the authentication a chain stored on the request, if any.
fn stored_authentication(
    request: &dyn next_web_core::http::HttpRequest,
) -> Option<Arc<dyn crate::core::Authentication>> {
    use next_web_core::any::AnyValue;

    let AnyValue::BoxedValue(value) = request.get_attribute(
        crate::web::context::RequestAttributeSecurityContextRepository::DEFAULT_REQUEST_ATTR_NAME,
    )?
    else {
        return None;
    };
    let context = (**value)
        .as_any()
        .downcast_ref::<Arc<dyn crate::core::context::SecurityContext>>()?;

    context.get_authentication()
}

/// Runs a request through the chain of a resource server that decodes with the
/// test decoder, and returns the status of the answer and the authentication the
/// chain stored.
async fn run(
    authorization: Option<&str>,
) -> (
    usize,
    Option<Arc<dyn crate::core::Authentication>>,
) {
    let mut http = http_security();
    http.oauth2_resource_server(|resource_server| {
        resource_server.jwt(|jwt| {
            jwt.decoder(Arc::new(TestDecoder));
        });
    });
    let chain = http.build();
    let filters = Arc::new(chain.get_filters().to_vec());
    let mut request = request(authorization);
    let mut response = axum::response::Response::default();
    Chain::new(filters)
        .run(&mut request, &mut response)
        .await
        .expect("the chain runs");

    (
        response.status().as_u16() as usize,
        stored_authentication(&request),
    )
}

#[test]
fn the_configuration_installs_the_filter() {
    let mut http = http_security();
    http.oauth2_resource_server(|resource_server| {
        resource_server.jwt(|jwt| {
            jwt.decoder(Arc::new(TestDecoder));
        });
    });

    let names = filter_names(http);

    assert!(
        names
            .iter()
            .any(|name| name == "BearerTokenAuthenticationFilter"),
        "the filters were: {names:?}"
    );
}

#[tokio::test]
async fn a_bearer_token_authenticates_the_request() {
    let (status, authentication) = run(Some("Bearer good-token")).await;

    let authentication = authentication.expect("the bearer token authenticates the request");
    assert_eq!(authentication.name(), "Alice", "the subject names the user");
    let authorities = authentication
        .authorities()
        .iter()
        .filter_map(|authority| authority.authority())
        .map(|authority| authority.to_string())
        .collect::<Vec<_>>();
    assert!(
        authorities.iter().any(|authority| authority == "SCOPE_read"),
        "the scopes of the token become authorities, the authorities were: {authorities:?}"
    );
    assert_eq!(
        authentication.of(),
        std::any::TypeId::of::<
            crate::oauth2::server::resource::bearer::BearerTokenAuthenticationToken,
        >(),
        "the stored authentication is the bearer token one"
    );
    assert_eq!(status, 200, "the request itself is answered by the chain");
}

#[tokio::test]
async fn a_request_without_a_bearer_token_is_not_authenticated() {
    let (status, authentication) = run(None).await;

    assert!(
        authentication.is_none(),
        "a request without a bearer token is not authenticated"
    );
    assert_eq!(
        status, 200,
        "the chain leaves such a request to the application"
    );
}

#[tokio::test]
async fn a_token_that_is_refused_is_answered_with_unauthorized() {
    let (status, authentication) = run(Some("Bearer another-token")).await;

    assert!(
        authentication.is_none(),
        "a token the decoder refuses authenticates nobody"
    );
    assert_eq!(
        status, 401,
        "a bearer token that is refused is answered with 401 rather than being ignored"
    );
}
