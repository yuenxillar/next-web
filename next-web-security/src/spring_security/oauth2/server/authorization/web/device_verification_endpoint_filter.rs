//! The page on which the resource owner answers a device authorization.

use std::sync::Arc;

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

use super::support::{error_response, json_response, resource_owner_required};
use crate::{
    authentication::AuthenticationTrustResolverImpl,
    authorization::AuthenticationTrustResolver,
    core::{context::SecurityContextHolder, Authentication},
    oauth2::server::authorization::{
        client_authentication::form_parameter, context::AuthorizationServerContext,
        error::OAuth2Error, token::now,
    },
};

/// Serves the page on which the resource owner answers a device authorization.
///
/// The resource owner reads the user code of a device, enters it on the page,
/// and approves or denies the request. The device then receives the tokens of
/// the authorization when it polls the token endpoint.
#[derive(Clone)]
pub struct OAuth2DeviceVerificationEndpointFilter {
    context: AuthorizationServerContext,
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl OAuth2DeviceVerificationEndpointFilter {
    /// Creates the filter of the page.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self {
            context,
            trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
        }
    }

    /// Returns the resource owner of the request, if one is logged in.
    fn resource_owner(&self) -> Option<Arc<dyn Authentication>> {
        SecurityContextHolder::get_context()
            .get_authentication()
            .filter(|authentication| !self.trust_resolver.is_anonymous(Some(&**authentication)))
    }

    /// Serves the answer of the resource owner.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the page.
    /// * `response` - The response of the page.
    fn verify(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(user_code) = form_parameter(request, "user_code") else {
            error_response(
                response,
                &OAuth2Error::invalid_request("The user_code parameter is required"),
            );
            return Ok(());
        };

        let Some(authorization) = self
            .context
            .device_authorizations()
            .find_by_user_code(&user_code)
        else {
            error_response(
                response,
                &OAuth2Error::invalid_request("The user code is unknown or expired"),
            );
            return Ok(());
        };
        if authorization.is_expired(now()) {
            error_response(
                response,
                &OAuth2Error::invalid_request("The user code is unknown or expired"),
            );
            return Ok(());
        }

        // The resource owner has to be logged in before the request is
        // answered, because the tokens of the authorization are issued for that
        // resource owner.
        let Some(resource_owner) = self.resource_owner() else {
            return Err(resource_owner_required());
        };

        if request.method() != HttpMethod::POST {
            // The page reports the request of the device, so the application
            // can show what the resource owner is asked to approve.
            json_response(
                response,
                StatusCode::OK,
                &json!({
                    "user_code": authorization.user_code(),
                    "client_id": authorization.registered_client_id(),
                    "scope": authorization.scopes().join(" "),
                    "status": status_of(&authorization),
                }),
            );
            return Ok(());
        }

        let mut authorization = authorization;
        if form_parameter(request, "deny").is_some() {
            authorization.deny();
        } else {
            authorization.approve(resource_owner.name().to_string());
        }
        self.context
            .device_authorizations()
            .save(authorization.clone());

        json_response(
            response,
            StatusCode::OK,
            &json!({
                "user_code": authorization.user_code(),
                "status": status_of(&authorization),
            }),
        );

        Ok(())
    }
}

#[async_trait]
impl HttpFilter for OAuth2DeviceVerificationEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().device_verification_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        self.verify(request, response)
    }
}

impl Named for OAuth2DeviceVerificationEndpointFilter {
    fn name(&self) -> &str {
        "OAuth2DeviceVerificationEndpointFilter"
    }
}

/// Returns the state of the authorization as the value of the response.
///
/// # Arguments
///
/// * `authorization` - The authorization of the device.
fn status_of(
    authorization: &crate::oauth2::server::authorization::device::OAuth2DeviceAuthorization,
) -> &'static str {
    use crate::oauth2::server::authorization::device::DeviceAuthorizationStatus;

    match authorization.status() {
        DeviceAuthorizationStatus::Pending => "pending",
        DeviceAuthorizationStatus::Approved => "approved",
        DeviceAuthorizationStatus::Denied => "denied",
    }
}
