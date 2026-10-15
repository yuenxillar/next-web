//! The endpoint that starts a device authorization.

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
    client_authentication::{authenticate_client, form_parameter},
    context::AuthorizationServerContext,
    device::OAuth2DeviceAuthorization,
    error::OAuth2Error,
    token::now,
};

/// The lifetime of a device authorization, in seconds.
pub const DEVICE_AUTHORIZATION_TIME_TO_LIVE: u64 = 300;

/// The number of seconds between the polls of a device.
pub const DEVICE_AUTHORIZATION_INTERVAL: u64 = 5;

/// Serves the device authorization endpoint of the authorization server.
///
/// A device that cannot show a browser asks the endpoint for a device code and
/// a user code. The device polls the token endpoint with its device code, and
/// the resource owner approves the request on the verification page of the
/// application with the user code.
#[derive(Clone)]
pub struct OAuth2DeviceAuthorizationEndpointFilter {
    context: AuthorizationServerContext,
}

impl OAuth2DeviceAuthorizationEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }

    /// Starts a device authorization and answers with its codes.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn authorize(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request("The device authorization endpoint supports POST"),
            );
            return Ok(());
        }

        match self.device_authorization(request, response) {
            Ok(()) => {}
            Err(error) => error_response(response, &error),
        }

        Ok(())
    }

    /// Stores the device authorization of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn device_authorization(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), OAuth2Error> {
        let registered_client = authenticate_client(request, &self.context)?;

        let scopes = match form_parameter(request, "scope") {
            Some(scope) => scope
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            None => registered_client.scopes().to_vec(),
        };
        for scope in &scopes {
            if !registered_client.has_scope(scope) {
                return Err(OAuth2Error::invalid_scope(format!(
                    "The {scope} scope is not registered for the client"
                )));
            }
        }

        let device_code = next_web_core::Uuid::new_v4().to_string();
        let user_code = generate_user_code();
        let expires_at = now() + DEVICE_AUTHORIZATION_TIME_TO_LIVE;
        self.context
            .device_authorizations()
            .save(OAuth2DeviceAuthorization::new(
                device_code.clone(),
                user_code.clone(),
                registered_client.id_value(),
                scopes,
                expires_at,
                DEVICE_AUTHORIZATION_INTERVAL,
            ));

        let issuer = self.context.issuer();
        let verification_uri = format!(
            "{issuer}{}",
            self.context.settings().device_verification_endpoint()
        );
        json_response(
            response,
            StatusCode::OK,
            &json!({
                "device_code": device_code,
                "user_code": user_code,
                "verification_uri": verification_uri,
                "verification_uri_complete":
                    format!("{verification_uri}?user_code={user_code}"),
                "expires_in": DEVICE_AUTHORIZATION_TIME_TO_LIVE,
                "interval": DEVICE_AUTHORIZATION_INTERVAL,
            }),
        );

        Ok(())
    }
}

#[async_trait]
impl HttpFilter for OAuth2DeviceAuthorizationEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().device_authorization_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        self.authorize(request, response)
    }
}

impl Named for OAuth2DeviceAuthorizationEndpointFilter {
    fn name(&self) -> &str {
        "OAuth2DeviceAuthorizationEndpointFilter"
    }
}

/// Returns a user code the resource owner can enter.
///
/// The alphabet leaves out the characters that a resource owner confuses when
/// the code is read from a small screen.
pub fn generate_user_code() -> String {
    const ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";

    let bytes = *next_web_core::Uuid::new_v4().as_bytes();
    let code = bytes
        .iter()
        .take(8)
        .map(|byte| ALPHABET[*byte as usize % ALPHABET.len()] as char)
        .collect::<String>();

    format!("{}-{}", &code[..4], &code[4..])
}

#[cfg(test)]
mod tests {
    use super::generate_user_code;

    #[test]
    fn a_user_code_is_two_groups_of_four_characters() {
        let code = generate_user_code();

        assert_eq!(code.len(), 9);
        assert_eq!(&code[4..5], "-");
        assert!(code
            .chars()
            .filter(|character| *character != '-')
            .all(|character| character.is_ascii_uppercase()));
    }

    #[test]
    fn two_user_codes_are_different() {
        assert_ne!(generate_user_code(), generate_user_code());
    }
}
