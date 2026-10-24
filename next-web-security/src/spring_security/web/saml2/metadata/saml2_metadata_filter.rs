use std::sync::Arc;

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse, StatusCode,
    },
    Named,
};

use crate::{
    saml2::{
        metadata::Saml2Metadata,
        registration::RelyingPartyRegistrationRepository,
    },
    web::util::matcher::{OrRequestMatcher, PathPatternRequestMatcher, RequestMatcher},
};

/// The endpoints a metadata request is served on by default.
///
/// The path without a registration id answers with every registration, and the
/// two paths with one answer with that registration. The
/// `service-provider-metadata` path is the older of the two spellings and is
/// kept for the asserting parties configured against it.
const DEFAULT_METADATA_PATTERNS: &[&str] = &[
    "/saml2/metadata/{registrationId}",
    "/saml2/metadata",
    "/saml2/service-provider-metadata/{registrationId}",
];

/// Serves the metadata of the relying party registrations of an application.
///
/// The document describes the service provider to an asserting party: where it
/// posts an assertion, where it receives a logout request, and which certificate
/// it signs with. A request that carries a registration id is answered with that
/// registration, a request without one with every registration.
#[derive(Clone)]
pub struct Saml2MetadataFilter {
    registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    metadata: Saml2Metadata,
    request_matcher: Arc<dyn RequestMatcher>,
    metadata_filename: String,
}

impl Saml2MetadataFilter {
    /// The name the served document is offered under, by default.
    pub const DEFAULT_METADATA_FILE_NAME: &'static str = "saml-{registrationId}-metadata.xml";

    /// Creates a filter over the given registrations.
    pub fn new(registrations: Arc<dyn RelyingPartyRegistrationRepository>) -> Self {
        Self {
            registrations,
            metadata: Saml2Metadata,
            request_matcher: Arc::new(OrRequestMatcher::new(
                DEFAULT_METADATA_PATTERNS
                    .iter()
                    .map(|pattern| {
                        Arc::new(PathPatternRequestMatcher::path_pattern(None, pattern))
                            as Arc<dyn RequestMatcher>
                    })
                    .collect(),
            )),
            metadata_filename: Self::DEFAULT_METADATA_FILE_NAME.to_string(),
        }
    }

    /// Uses the given matcher to decide which requests this filter answers.
    pub fn set_request_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) {
        self.request_matcher = request_matcher;
    }

    /// Sets the file name the document is offered under.
    ///
    /// # Panics
    ///
    /// Panics when the name is empty or carries no `{registrationId}` match
    /// variable, because the name of a document that describes one registration
    /// has to name it.
    pub fn set_metadata_filename(&mut self, metadata_filename: &str) {
        assert!(
            !metadata_filename.trim().is_empty(),
            "metadataFilename cannot be empty"
        );
        assert!(
            metadata_filename.contains("{registrationId}"),
            "metadataFilename must contain a {{registrationId}} match variable"
        );

        self.metadata_filename = metadata_filename.to_string();
    }

    /// Writes the document, and the headers it is offered with, to the response.
    fn write_metadata(&self, response: &mut dyn HttpResponse, document: &str, file_name: &str) {
        response.insert_header(
            "Content-Type",
            "application/samlmetadata+xml;charset=UTF-8",
        );
        response.insert_header(
            "Content-Disposition",
            &format!(
                "attachment; filename=\"{file_name}\"; filename*=UTF-8''{}",
                urlencoding::encode(file_name)
            ),
        );
        response.insert_header("Content-Length", &document.len().to_string());
        response.set_body(document.to_string().into_bytes());
    }
}

impl Named for Saml2MetadataFilter {
    fn name(&self) -> &str {
        "Saml2MetadataFilter"
    }
}

#[async_trait]
impl HttpFilter for Saml2MetadataFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let matcher = self.request_matcher.matcher(request);
        if !matcher.is_match() {
            return filter_chain.do_filter(request, response).await;
        }

        let base_url = crate::web::saml2::request_base_url(request);
        let registration_id = matcher
            .get_variables()
            .and_then(|variables| variables.get("registrationId"))
            .cloned();

        let (document, file_name) = match registration_id {
            Some(registration_id) => {
                let Some(registration) = self
                    .registrations
                    .find_by_registration_id(&registration_id)
                else {
                    // A request for a registration the application does not have
                    // is answered with the status the reference implementation
                    // uses, and with nothing else.
                    response.set_status_code(StatusCode::UNAUTHORIZED);
                    return Ok(());
                };

                let document = self.metadata.resolve(&registration, &base_url);
                let file_name = self
                    .metadata_filename
                    .replace("{registrationId}", registration.registration_id());

                (document, file_name)
            }
            None => {
                let document = self.metadata.resolve_all(&self.registrations.iter(), &base_url);
                // A document that describes every registration is not named
                // after one of them.
                let file_name = self
                    .metadata_filename
                    .replace("{registrationId}", &format!("{:016x}", rand::random::<u64>()));

                (document, file_name)
            }
        };

        self.write_metadata(response, &document, &file_name);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse,
        },
    };

    use super::*;
    use crate::saml2::registration::{
        InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
    };

    /// Returns a `GET` request for an absolute URL.
    fn request(url: &str) -> Request {
        Request::builder()
            .method("GET")
            .uri(url)
            .body(Body::empty())
            .expect("a valid request")
    }

    /// Returns a registration whose endpoints are templates.
    fn registration(registration_id: &str) -> RelyingPartyRegistration {
        RelyingPartyRegistration::with_registration_id(registration_id).build()
    }

    /// Returns a filter over the given registrations.
    fn filter(registrations: Vec<RelyingPartyRegistration>) -> Saml2MetadataFilter {
        Saml2MetadataFilter::new(Arc::new(InMemoryRelyingPartyRegistrationRepository::new(
            registrations,
        )))
    }

    /// Records whether the rest of the chain ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<AtomicBool>,
    }

    impl RecordingChain {
        /// Returns whether the chain ran.
        fn ran(&self) -> bool {
            self.ran.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.ran.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Runs the filter against the request, returning the response.
    async fn run(filter: &Saml2MetadataFilter, url: &str) -> Response {
        let mut request = request(url);
        let mut response = Response::default();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("the filter runs");

        response
    }

    /// Returns a header of the response, if it is present.
    fn header<'a>(response: &'a Response, name: &str) -> Option<&'a str> {
        response.headers().get(name).and_then(|value| value.to_str().ok())
    }

    #[tokio::test]
    async fn the_registration_metadata_is_served() {
        let filter = filter(vec![registration("okta")]);

        let response = run(&filter, "http://sp.example.org/saml2/metadata/okta").await;

        assert_eq!(response.status(), 200);
        assert_eq!(
            header(&response, "content-type"),
            Some("application/samlmetadata+xml;charset=UTF-8")
        );
        assert_eq!(
            header(&response, "content-disposition"),
            Some("attachment; filename=\"saml-okta-metadata.xml\"; filename*=UTF-8''saml-okta-metadata.xml")
        );
    }

    #[tokio::test]
    async fn the_legacy_endpoint_serves_the_same_document() {
        let filter = filter(vec![registration("okta")]);

        let response =
            run(&filter, "http://sp.example.org/saml2/service-provider-metadata/okta").await;

        assert_eq!(response.status(), 200);
        assert_eq!(
            header(&response, "content-disposition"),
            Some("attachment; filename=\"saml-okta-metadata.xml\"; filename*=UTF-8''saml-okta-metadata.xml")
        );
    }

    #[tokio::test]
    async fn every_registration_is_served_without_an_id() {
        let filter = filter(vec![registration("okta"), registration("azure")]);

        let response = run(&filter, "http://sp.example.org/saml2/metadata").await;

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn an_unknown_registration_is_unauthorized() {
        let filter = filter(vec![registration("okta")]);

        let response = run(&filter, "http://sp.example.org/saml2/metadata/azure").await;

        assert_eq!(response.status(), 401);
        assert_eq!(header(&response, "content-type"), None);
        assert_eq!(header(&response, "content-disposition"), None);
    }

    #[tokio::test]
    async fn a_request_that_does_not_match_is_passed_down_the_chain() {
        let filter = filter(vec![registration("okta")]);
        let mut request = request("http://sp.example.org/saml2/authenticate/okta");
        let mut response = Response::default();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("the filter runs");

        assert!(chain.ran(), "the rest of the chain handles the request");
        assert_eq!(response.status(), 200);
    }

    #[test]
    #[should_panic(expected = "metadataFilename must contain a {registrationId} match variable")]
    fn a_metadata_filename_without_the_registration_id_is_rejected() {
        filter(vec![registration("okta")]).set_metadata_filename("metadata.xml");
    }
}
