//! The endpoint the asserting party answers a logout of the application on.

use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        HttpMethod, HttpRequest, HttpResponse, StatusCode,
        filter::{FilterError, HttpFilter, HttpFilterChain},
    },
    Named,
};

use crate::{
    saml2::{
        error::{Saml2Error, Saml2ErrorKind},
        logout::{LOGOUT_RESPONSE_PARAMETER, Saml2LogoutResponse},
        registration::{RelyingPartyRegistration, RelyingPartyRegistrationRepository},
        xml_dsig,
    },
    web::{RedirectStrategy, authentication::logout::LogoutHandler},
};

/// The endpoint the logout response of the asserting party is served on.
pub const DEFAULT_LOGOUT_RESPONSE_URL: &str = "/logout/saml2/slo";

/// Ends the session of the application when the asserting party answers its logout.
///
/// The filter serves the single logout endpoint of a registration for a
/// `SAMLResponse`: it reads the message, verifies the signature of the asserting
/// party on it, refuses a response that does not report success, ends the session
/// through the configured [`LogoutHandler`]s, and sends the user to the success URL
/// of the logout.
#[derive(Clone)]
pub struct Saml2LogoutResponseFilter {
    registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    logout_handlers: Arc<Vec<Arc<dyn LogoutHandler>>>,
    logout_response_url: Box<str>,
    success_url: Box<str>,
}

impl Saml2LogoutResponseFilter {
    /// Creates the filter over the registrations of an application.
    pub fn new(
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
        logout_handlers: Arc<Vec<Arc<dyn LogoutHandler>>>,
    ) -> Self {
        Self {
            registrations,
            logout_handlers,
            logout_response_url: DEFAULT_LOGOUT_RESPONSE_URL.into(),
            success_url: "/login?logout".into(),
        }
    }

    /// Sets the path the endpoint of the filter listens on.
    pub fn set_logout_response_url(&mut self, logout_response_url: impl Into<Box<str>>) {
        self.logout_response_url = logout_response_url.into();
    }

    /// Sets the URL the user is sent to once the logout is complete.
    pub fn set_logout_success_url(&mut self, success_url: impl Into<Box<str>>) {
        self.success_url = success_url.into();
    }

    /// Returns the registration whose logout endpoint serves the request.
    fn registration_for(&self, request: &dyn HttpRequest) -> Option<RelyingPartyRegistration> {
        let base_url = super::saml2_logout_request_filter::base_url(request);
        self.registrations.iter().into_iter().find(|registration| {
            let location = registration
                .single_logout_service_location()
                .map(|location| {
                    crate::saml2::xml::resolve_placeholders(location, registration, &base_url)
                })
                .unwrap_or_default();

            location == format!("{base_url}{}", self.logout_response_url)
        })
    }

    /// Ends the session of the request, and sends the user on.
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(value) = request.form_parameter(LOGOUT_RESPONSE_PARAMETER) else {
            return Ok(());
        };
        let Some(registration) = self.registration_for(request) else {
            return refuse(response, "no registration serves the logout endpoint");
        };
        let message = match Saml2LogoutResponse::from_parameter(
            &value,
            registration.single_logout_service_binding(),
        ) {
            Ok(message) => message,
            Err(error) => return refuse(response, error.message()),
        };
        if let Err(error) = verify(&registration, &message) {
            return refuse(response, error.message());
        }
        if !message.is_success() {
            return refuse(response, "the asserting party did not report a successful logout");
        }

        for logout_handler in self.logout_handlers.iter() {
            logout_handler.logout(request, response, None).await;
        }

        RedirectStrategy::send_redirect(
            &crate::web::DefaultRedirectStrategy::default(),
            request,
            response,
            &self.success_url,
        )
        .map_err(FilterError::from)
    }
}

/// Verifies the signature of a logout response.
fn verify(
    registration: &RelyingPartyRegistration,
    response: &Saml2LogoutResponse,
) -> Result<(), Saml2Error> {
    if !response.is_signed() {
        return Err(Saml2Error::new(
            Saml2ErrorKind::VerificationFailed,
            "the logout response carries no signature",
        ));
    }
    let Some(public_key) = registration.verification_public_key() else {
        return Err(Saml2Error::new(
            Saml2ErrorKind::VerificationFailed,
            "the registration holds no key of the asserting party, so no logout response can be \
             verified",
        ));
    };

    xml_dsig::verify_signature(response.document(), public_key)
}

/// Answers a request the profile refuses.
fn refuse(response: &mut dyn HttpResponse, reason: &str) -> Result<(), FilterError> {
    tracing::debug!("the logout response is refused: {reason}");
    response.set_status_code(StatusCode::BAD_REQUEST);
    let mut body = String::from(r#"{"error":"invalid_request","error_description":""#);
    body.push_str(&reason.replace('"', "'"));
    body.push_str(r#""}"#);
    let _ = response.set_body(body.into_bytes());

    Ok(())
}

#[async_trait]
impl HttpFilter for Saml2LogoutResponseFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let serves = matches!(request.method(), HttpMethod::POST | HttpMethod::GET)
            && request.form_parameter(LOGOUT_RESPONSE_PARAMETER).is_some();
        if !serves {
            return filter_chain.do_filter(request, response).await;
        }

        self.logout(request, response).await
    }

    fn supports(&self, name: &str) -> bool {
        name == "Saml2LogoutResponseFilter"
    }
}

impl Named for Saml2LogoutResponseFilter {
    fn name(&self) -> &str {
        "Saml2LogoutResponseFilter"
    }
}
