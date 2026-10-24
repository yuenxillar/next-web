//! The endpoint the asserting party sends a logout request to.

use std::{io::Write, sync::Arc};

use base64::{Engine, engine::general_purpose::STANDARD};
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
        logout::{LOGOUT_RESPONSE_PARAMETER, LOGOUT_REQUEST_PARAMETER, Saml2LogoutRequest},
        registration::{RelyingPartyRegistration, RelyingPartyRegistrationRepository, Saml2MessageBinding},
        xml_dsig,
    },
    web::{RedirectStrategy, authentication::logout::LogoutHandler},
};

/// The endpoint the logout request of the asserting party is served on.
pub const DEFAULT_LOGOUT_REQUEST_URL: &str = "/logout/saml2/slo";

/// Ends the session the asserting party asks to end, and answers the request.
///
/// The filter serves the single logout endpoint of a registration: it reads the
/// `SAMLRequest` of the request, verifies the signature of the asserting party on
/// it, ends the session through the configured [`LogoutHandler`]s, and answers the
/// asserting party with a `LogoutResponse` in the location its registration names.
/// A request that is not from the asserting party — no signature, a signature of
/// another party, or a message that is not a logout request — is answered with
/// 400 and ends nothing.
#[derive(Clone)]
pub struct Saml2LogoutRequestFilter {
    registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    logout_handlers: Arc<Vec<Arc<dyn LogoutHandler>>>,
    logout_request_url: Box<str>,
    success_url: Box<str>,
}

impl Saml2LogoutRequestFilter {
    /// Creates the filter over the registrations of an application.
    pub fn new(
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
        logout_handlers: Arc<Vec<Arc<dyn LogoutHandler>>>,
    ) -> Self {
        Self {
            registrations,
            logout_handlers,
            logout_request_url: DEFAULT_LOGOUT_REQUEST_URL.into(),
            success_url: "/login?logout".into(),
        }
    }

    /// Sets the path the endpoint of the filter listens on.
    pub fn set_logout_request_url(&mut self, logout_request_url: impl Into<Box<str>>) {
        self.logout_request_url = logout_request_url.into();
    }

    /// Sets the URL a logout ends on when the asserting party is not answered.
    pub fn set_logout_success_url(&mut self, success_url: impl Into<Box<str>>) {
        self.success_url = success_url.into();
    }

    /// Returns the registration whose logout endpoint serves the request.
    fn registration_for(&self, request: &dyn HttpRequest) -> Option<RelyingPartyRegistration> {
        let base_url = base_url(request);
        self.registrations.iter().into_iter().find(|registration| {
            let location = registration
                .single_logout_service_location()
                .map(|location| resolve(location, registration, &base_url))
                .unwrap_or_default();

            location == format!("{base_url}{}", self.logout_request_url)
        })
    }

    /// Ends the session of the request, and answers the asserting party.
    async fn logout(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        let Some(value) = request.form_parameter(LOGOUT_REQUEST_PARAMETER) else {
            return Ok(());
        };
        let Some(registration) = self.registration_for(request) else {
            return refuse(response, "no registration serves the logout endpoint");
        };
        let message = match Saml2LogoutRequest::from_parameter(
            &value,
            registration.single_logout_service_binding(),
        ) {
            Ok(message) => message,
            Err(error) => return refuse(response, error.message()),
        };
        // The request is believed only when the asserting party signed it.
        if let Err(error) = verify(&registration, &message) {
            return refuse(response, error.message());
        }

        // The local session ends first, so a failure of the answer to the asserting
        // party does not leave the user signed in.
        for logout_handler in self.logout_handlers.iter() {
            logout_handler.logout(request, response, None).await;
        }

        let Some(response_location) = registration.single_logout_service_response_location() else {
            return redirect(response, request, &self.success_url);
        };
        let answer = logout_response_of(&registration, &message);
        let encoded = match encode(&answer, registration.single_logout_service_binding()) {
            Ok(encoded) => encoded,
            Err(error) => return refuse(response, error.message()),
        };
        redirect(
            response,
            request,
            &format!("{response_location}?{LOGOUT_RESPONSE_PARAMETER}={encoded}"),
        )
    }
}

/// Verifies the signature of a logout request.
pub(crate) fn verify(
    registration: &RelyingPartyRegistration,
    request: &Saml2LogoutRequest,
) -> Result<(), Saml2Error> {
    if !request.is_signed() {
        return Err(Saml2Error::new(
            Saml2ErrorKind::VerificationFailed,
            "the logout request carries no signature",
        ));
    }
    let Some(public_key) = registration.verification_public_key() else {
        return Err(Saml2Error::new(
            Saml2ErrorKind::VerificationFailed,
            "the registration holds no key of the asserting party, so no logout request can be \
             verified",
        ));
    };

    xml_dsig::verify_signature(request.document(), public_key)
}

/// Returns the logout response that answers a logout request.
fn logout_response_of(
    registration: &RelyingPartyRegistration,
    request: &Saml2LogoutRequest,
) -> Vec<u8> {
    let destination = registration
        .single_logout_service_response_location()
        .unwrap_or_default();
    let id = format!("SLO-{}", crate::saml2::logout::new_id());
    let body = format!(
        r#"<saml2p:LogoutResponse xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" Destination="{}" ID="{}" InResponseTo="{}" IssueInstant="{}" Version="2.0"><saml2:Issuer>{}</saml2:Issuer><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"></saml2p:StatusCode></saml2p:Status></saml2p:LogoutResponse>"#,
        crate::saml2::xml::escape_xml(destination),
        crate::saml2::xml::escape_xml(&id),
        crate::saml2::xml::escape_xml(request.id().unwrap_or_default()),
        issue_instant(),
        crate::saml2::xml::escape_xml(registration.entity_id()),
    );

    body.into_bytes()
}

/// Returns a message encoded for its binding.
pub(crate) fn encode(message: &[u8], binding: Saml2MessageBinding) -> Result<String, Saml2Error> {
    match binding {
        Saml2MessageBinding::Post => Ok(STANDARD.encode(message)),
        Saml2MessageBinding::Redirect => {
            let mut deflated = Vec::new();
            let mut encoder =
                flate2::write::DeflateEncoder::new(&mut deflated, flate2::Compression::default());
            encoder.write_all(message).map_err(|error| {
                Saml2Error::new(
                    Saml2ErrorKind::InvalidResponse,
                    format!("the message is not deflated: {error}"),
                )
            })?;
            encoder.finish().map_err(|error| {
                Saml2Error::new(
                    Saml2ErrorKind::InvalidResponse,
                    format!("the message is not deflated: {error}"),
                )
            })?;

            Ok(STANDARD.encode(&deflated))
        }
    }
}

/// Returns the base URL of a request.
pub(crate) fn base_url(request: &dyn HttpRequest) -> String {
    let scheme = if request.is_secure() { "https" } else { "http" };
    let host = request
        .header("host")
        .map(str::to_owned)
        .unwrap_or_else(|| "localhost".to_string());

    format!("{scheme}://{host}")
}

/// Returns the instant a SAML 2.0 message names, as the current time.
pub(crate) fn issue_instant() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Resolves the placeholders of a location for a request.
fn resolve(location: &str, registration: &RelyingPartyRegistration, base_url: &str) -> String {
    crate::saml2::xml::resolve_placeholders(location, registration, base_url)
}

/// Answers a request the profile refuses.
fn refuse(response: &mut dyn HttpResponse, reason: &str) -> Result<(), FilterError> {
    tracing::debug!("the logout request is refused: {reason}");
    response.set_status_code(StatusCode::BAD_REQUEST);
    let mut body = String::from(r#"{"error":"invalid_request","error_description":""#);
    body.push_str(&reason.replace('"', "'"));
    body.push_str(r#""}"#);
    let _ = response.set_body(body.into_bytes());

    Ok(())
}

/// Redirects the request.
fn redirect(
    response: &mut dyn HttpResponse,
    request: &mut dyn HttpRequest,
    location: &str,
) -> Result<(), FilterError> {
    RedirectStrategy::send_redirect(
        &crate::web::DefaultRedirectStrategy::default(),
        request,
        response,
        location,
    )
    .map_err(FilterError::from)
}

#[async_trait]
impl HttpFilter for Saml2LogoutRequestFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let serves = matches!(request.method(), HttpMethod::POST | HttpMethod::GET)
            && request.form_parameter(LOGOUT_REQUEST_PARAMETER).is_some();
        if !serves {
            return filter_chain.do_filter(request, response).await;
        }

        self.logout(request, response).await
    }

    fn supports(&self, name: &str) -> bool {
        name == "Saml2LogoutRequestFilter"
    }
}

impl Named for Saml2LogoutRequestFilter {
    fn name(&self) -> &str {
        "Saml2LogoutRequestFilter"
    }
}
