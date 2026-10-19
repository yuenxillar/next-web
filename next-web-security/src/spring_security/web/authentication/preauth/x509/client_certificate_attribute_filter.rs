//! The filter that hands the certificate of the connection to the X.509 filter.
//!
//! A certificate is not part of an HTTP request: the transport that made the
//! connection validated it and inserted it, and it is the X.509 filter that
//! reads the *attribute* a servlet-style container would have written. This
//! filter is the step in between, on every request: when the connection carries
//! a certificate, its subject is written into the attribute the filter reads, so
//! that a login over TLS works without the application wiring anything.

use std::sync::Arc;

use next_web_core::{
    any::AnyValue,
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    Named,
};

use super::{subject_dn::subject_dn, x509_authentication_filter::DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE};

/// Writes the subject of the certificate of the connection into the request
/// attribute the [`X509AuthenticationFilter`] reads.
///
/// The subject of the certificate is used as the transport rendered it when the
/// transport rendered one — a terminator that reports it, for instance — and read
/// out of the DER encoding otherwise. A request whose attribute is already set is
/// left alone, so an application that fills the attribute itself keeps doing so.
///
/// [`X509AuthenticationFilter`]: super::x509_authentication_filter::X509AuthenticationFilter
#[derive(Clone)]
pub struct ClientCertificateAttributeFilter {
    attribute: Arc<str>,
}

impl ClientCertificateAttributeFilter {
    /// Creates the filter over the attribute the X.509 filter reads by default.
    pub fn new() -> Self {
        Self {
            attribute: Arc::from(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE),
        }
    }

    /// Sets the request attribute the subject of the certificate is written to.
    pub fn set_client_certificate_attribute(&mut self, attribute: impl Into<Arc<str>>) {
        let attribute = attribute.into();
        assert!(
            !attribute.trim().is_empty(),
            "clientCertificateAttribute must not be empty or null"
        );
        self.attribute = attribute;
    }
}

impl Default for ClientCertificateAttributeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Named for ClientCertificateAttributeFilter {
    fn name(&self) -> &str {
        "ClientCertificateAttributeFilter"
    }
}

#[async_trait]
impl HttpFilter for ClientCertificateAttributeFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.get_attribute(&self.attribute).is_some() {
            return filter_chain.do_filter(request, response).await;
        }

        let Some(certificate) = request.peer_certificate() else {
            return filter_chain.do_filter(request, response).await;
        };

        // The transport may have rendered the subject already, and a rendered one
        // is what the container of the client's certificate would report.
        let subject = match certificate.subject() {
            Some(subject) => Some(subject.to_string()),
            None => match subject_dn(certificate.encoded()) {
                Ok(subject) => Some(subject),
                Err(error) => {
                    tracing::debug!("the subject of the client certificate is not read: {error}");
                    None
                }
            },
        };

        if let Some(subject) = subject {
            request.set_attribute(&self.attribute, AnyValue::from(subject));
        }

        filter_chain.do_filter(request, response).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::{
            filter::{FilterError, HttpFilterChain},
            HttpRequest, HttpResponse, PeerCertificate,
        },
    };

    use super::*;

    /// A chain that records whether it ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<AtomicBool>,
    }

    impl RecordingChain {
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

    /// Returns a request, with a certificate of the fixtures when asked for one.
    fn request(certificate: Option<PeerCertificate>) -> Request {
        let mut request = Request::builder()
            .uri("https://127.0.0.1/")
            .body(Body::empty())
            .expect("a valid request");
        if let Some(certificate) = certificate {
            request.extensions_mut().insert(certificate);
        }

        request
    }

    /// Returns the DER of the client certificate the TLS fixtures carry.
    fn client_certificate() -> Vec<u8> {
        use base64::Engine;

        let pem = include_str!("../../../../../../tests/fixtures/tls/client-cert.pem");
        let body = pem
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect::<String>();

        base64::engine::general_purpose::STANDARD
            .decode(body)
            .expect("the fixture is base64")
    }

    /// Runs the filter over the request.
    async fn run(filter: &ClientCertificateAttributeFilter, request: &mut Request) -> bool {
        let mut response = Response::default();
        let chain = RecordingChain::default();
        filter
            .do_filter(request, &mut response, &chain)
            .await
            .expect("the filter runs");

        chain.ran()
    }

    #[tokio::test]
    async fn the_subject_of_the_certificate_of_the_connection_is_written() {
        let filter = ClientCertificateAttributeFilter::new();
        let mut request = request(Some(PeerCertificate::new(client_certificate())));

        let ran = run(&filter, &mut request).await;

        assert!(ran, "the request continues down the chain");
        let subject = request
            .get_attribute(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE)
            .and_then(|value| value.as_string());
        assert_eq!(subject.as_deref(), Some("O=Example, CN=Alice"));
    }

    #[tokio::test]
    async fn a_request_without_a_certificate_is_left_alone() {
        let filter = ClientCertificateAttributeFilter::new();
        let mut request = request(None);

        let ran = run(&filter, &mut request).await;

        assert!(ran, "the request continues down the chain");
        assert!(
            request
                .get_attribute(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE)
                .is_none(),
            "nothing is written for a connection without a certificate"
        );
    }

    #[tokio::test]
    async fn the_subject_a_transport_rendered_is_used_as_it_is() {
        let filter = ClientCertificateAttributeFilter::new();
        let mut request = request(Some(PeerCertificate::with_subject(
            vec![0x30, 0x00],
            "CN=Alice",
        )));

        run(&filter, &mut request).await;

        assert_eq!(
            request
                .get_attribute(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE)
                .and_then(|value| value.as_string())
                .as_deref(),
            Some("CN=Alice")
        );
    }

    #[tokio::test]
    async fn an_attribute_that_is_already_set_is_kept() {
        let filter = ClientCertificateAttributeFilter::new();
        let mut request = request(Some(PeerCertificate::new(client_certificate())));
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=From The Terminator"),
        );

        run(&filter, &mut request).await;

        assert_eq!(
            request
                .get_attribute(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE)
                .and_then(|value| value.as_string())
                .as_deref(),
            Some("CN=From The Terminator"),
            "an attribute the application wrote is not replaced"
        );
    }

    #[tokio::test]
    async fn the_subject_of_the_connection_names_the_user_the_filter_authenticates() {
        use crate::web::authentication::preauth::x509::{
            SubjectX500PrincipalExtractor, X509Certificate, X509PrincipalExtractor,
        };

        let filter = ClientCertificateAttributeFilter::new();
        let mut request = request(Some(PeerCertificate::new(client_certificate())));
        run(&filter, &mut request).await;
        let subject = request
            .get_attribute(DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE)
            .and_then(|value| value.as_string())
            .expect("the attribute holds the subject");

        // The extractor the X.509 filter uses by default reads the principal out
        // of the attribute the bridge wrote.
        let principal = SubjectX500PrincipalExtractor::default()
            .extract_principal(&X509Certificate::new(subject))
            .expect("the subject names a common name");

        assert_eq!(principal, "Alice");
    }
}
