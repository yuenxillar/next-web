use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use tracing::debug;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    authorization::AuthenticationManager,
    core::AuthenticationError,
    web::authentication::{
        preauth::base_pre_authenticated_processing_filter::{
            BasePreAuthenticatedProcessingFilter, BasePreAuthenticatedProcessingFilterExt,
        },
        AuthPrincipal,
    },
};

use super::{
    subject_x500_principal_extractor::SubjectX500PrincipalExtractor,
    x509_certificate::X509Certificate, x509_principal_extractor::X509PrincipalExtractor,
};

/// Default request attribute holding the client certificate.
pub const DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE: &str = "next.httprequest.X509Certificate";

/// A pre-authentication filter that extracts the principal from an X.509 client
/// certificate.
///
/// The certificate is read from the [`DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE`] request
/// attribute. The attribute may contain either the subject distinguished name as a string
/// or an [`X509Certificate`] object.
#[derive(Clone)]
pub struct X509AuthenticationFilter {
    principal_extractor: Arc<dyn X509PrincipalExtractor>,
    client_certificate_attribute: String,

    base: BasePreAuthenticatedProcessingFilter,
}

impl X509AuthenticationFilter {
    /// Creates a new filter using the supplied authentication manager.
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            base: BasePreAuthenticatedProcessingFilter::new(authentication_manager),
            principal_extractor: Arc::new(SubjectX500PrincipalExtractor::new()),
            client_certificate_attribute: DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE.to_string(),
        }
    }

    /// Sets the extractor used to obtain the principal from the certificate.
    pub fn set_principal_extractor(
        &mut self,
        principal_extractor: Arc<dyn X509PrincipalExtractor>,
    ) {
        self.principal_extractor = principal_extractor;
    }

    /// Sets the request attribute that holds the client certificate.
    pub fn set_client_certificate_attribute(
        &mut self,
        client_certificate_attribute: impl Into<String>,
    ) {
        let client_certificate_attribute = client_certificate_attribute.into();
        assert!(
            !client_certificate_attribute.trim().is_empty(),
            "clientCertificateAttribute must not be empty or null"
        );
        self.client_certificate_attribute = client_certificate_attribute;
    }
}

impl BasePreAuthenticatedProcessingFilterExt for X509AuthenticationFilter {
    /// Extracts the pre-authenticated principal from the client certificate, or `None`
    /// when no client certificate is present.
    fn get_pre_authenticated_principal(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        match extract_client_certificate(request, &self.client_certificate_attribute) {
            Some(certificate) => self
                .principal_extractor
                .extract_principal(&certificate)
                .map(|principal| Some(Arc::new(principal) as AuthPrincipal)),
            None => Ok(None),
        }
    }

    /// Extracts the pre-authenticated credentials from the client certificate, or `None`
    /// when no client certificate is present.
    fn get_pre_authenticated_credentials(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<AuthPrincipal>, AuthenticationError> {
        Ok(
            extract_client_certificate(request, &self.client_certificate_attribute)
                .map(|certificate| Arc::new(certificate.subject_dn().to_string()) as AuthPrincipal),
        )
    }
}

#[async_trait]
impl HttpFilter for X509AuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.base
            .do_filter(self, request, response, filter_chain)
            .await
    }
}

impl Named for X509AuthenticationFilter {
    fn name(&self) -> &str {
        "X509AuthenticationFilter"
    }
}

impl Deref for X509AuthenticationFilter {
    type Target = BasePreAuthenticatedProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for X509AuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

fn extract_client_certificate(
    request: &dyn HttpRequest,
    attribute: &str,
) -> Option<X509Certificate> {
    let Some(value) = request.get_attribute(attribute) else {
        debug!("No client certificate found in request.");
        return None;
    };

    if let Some(subject_dn) = value.as_string() {
        let certificate = X509Certificate::new(subject_dn);
        debug!("X.509 client authentication certificate: {}", certificate);
        return Some(certificate);
    }

    let certificate = value.as_object::<X509Certificate>();
    match &certificate {
        Some(certificate) => {
            debug!("X.509 client authentication certificate: {}", certificate)
        }
        None => debug!("No client certificate found in request."),
    }
    certificate
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{body::Body, extract::Request as AxumRequest, response::Response as AxumResponse};
    use next_web_core::{anys::any_value::AnyValue, async_trait};
    use tokio::sync::Mutex;

    use crate::authentication::authentication_details_source::AuthenticationDetailsSource;
    use crate::core::{
        authority::AuthorityUtils, context::SecurityContextHolder, Authentication,
        AuthenticationError, AuthenticationErrorKind,
    };
    use crate::web::authentication::preauth::pre_authenticated_authentication_token::PreAuthenticatedAuthenticationToken;
    use crate::web::authentication::{AuthPrincipal, Identity};

    use super::*;

    #[derive(Clone)]
    struct MockAuthenticationManager;

    #[async_trait]
    impl AuthenticationManager for MockAuthenticationManager {
        async fn authenticate(
            &self,
            authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            let principal = authentication.name().to_string();
            let credentials = authentication.credentials().map(ToString::to_string);
            Ok(Arc::new(
                PreAuthenticatedAuthenticationToken::with_authorities(
                    Arc::new(principal),
                    credentials.map(|s| Arc::new(s) as AuthPrincipal),
                    AuthorityUtils::create_authority_list(["FACTOR_X509"]),
                ),
            ))
        }
    }

    #[derive(Clone)]
    struct FailingAuthenticationManager;

    #[async_trait]
    impl AuthenticationManager for FailingAuthenticationManager {
        async fn authenticate(
            &self,
            _authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            Err(AuthenticationError::with_kind(
                "rejected",
                AuthenticationErrorKind::BadCredentials,
            ))
        }
    }

    #[derive(Clone, Default)]
    struct MarkerDetails;

    impl std::fmt::Display for MarkerDetails {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("marker-details")
        }
    }

    impl Identity for MarkerDetails {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[derive(Clone, Default)]
    struct MarkerDetailsSource;

    impl AuthenticationDetailsSource for MarkerDetailsSource {
        fn build_details(&self, _context: &dyn HttpRequest) -> AuthPrincipal {
            Arc::new(MarkerDetails)
        }
    }

    #[derive(Clone)]
    struct CapturingAuthenticationManager {
        details: Arc<Mutex<Option<String>>>,
    }

    #[async_trait]
    impl AuthenticationManager for CapturingAuthenticationManager {
        async fn authenticate(
            &self,
            authentication: &dyn Authentication,
        ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
            *self.details.lock().await = authentication.details().map(ToString::to_string);
            let principal = authentication.name().to_string();
            let credentials = authentication.credentials().map(ToString::to_string);
            Ok(Arc::new(
                PreAuthenticatedAuthenticationToken::with_authorities(
                    Arc::new(principal),
                    credentials.map(|s| Arc::new(s) as AuthPrincipal),
                    AuthorityUtils::create_authority_list(["FACTOR_X509"]),
                ),
            ))
        }
    }

    #[derive(Clone, Default)]
    struct RecordingChain {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn HttpRequest,
            _response: &mut dyn HttpResponse,
        ) -> Result<(), FilterError> {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    fn test_request() -> AxumRequest {
        let mut request = AxumRequest::builder().body(Body::empty()).unwrap();
        request.ready();
        request
    }

    fn test_response() -> AxumResponse {
        AxumResponse::new(Body::empty())
    }

    #[tokio::test]
    async fn authenticates_when_certificate_is_present() {
        SecurityContextHolder::clear_context();
        let filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Luke Taylor,OU=Dev,O=Monkey Machine"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert!(chain.called.load(Ordering::SeqCst));
        assert!(SecurityContextHolder::get_context()
            .get_authentication()
            .is_some());
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn continues_without_authenticating_when_certificate_is_missing() {
        SecurityContextHolder::clear_context();
        let filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        let mut request = test_request();
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert!(chain.called.load(Ordering::SeqCst));
        assert!(SecurityContextHolder::get_context()
            .get_authentication()
            .is_none());
    }

    #[test]
    fn extracts_credentials_from_subject_dn() {
        let filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Duke,O=Example"),
        );

        let credentials = filter
            .get_pre_authenticated_credentials(&request)
            .expect("credentials should be extracted")
            .map(|credentials| credentials.to_string());

        assert_eq!(credentials, Some("CN=Duke,O=Example".to_string()));
    }

    #[test]
    fn fails_when_certificate_has_no_matching_attribute() {
        let filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("O=Example,C=US"),
        );

        let error = filter
            .get_pre_authenticated_principal(&request)
            .err()
            .expect("extraction should fail");

        assert_eq!(
            error.kind(),
            crate::core::AuthenticationErrorKind::BadCredentials
        );
    }

    #[tokio::test]
    async fn reads_certificate_from_object_attribute() {
        SecurityContextHolder::clear_context();
        let filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::Object(Box::new(X509Certificate::new("CN=Alice,O=Example"))),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert!(SecurityContextHolder::get_context()
            .get_authentication()
            .is_some());
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn reads_certificate_from_custom_attribute() {
        SecurityContextHolder::clear_context();
        let mut filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        filter.set_client_certificate_attribute("x-client-cert");
        let mut request = test_request();
        request.set_attribute("x-client-cert", AnyValue::from("CN=Bob,O=Example"));
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert!(SecurityContextHolder::get_context()
            .get_authentication()
            .is_some());
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn supports_custom_principal_extractor() {
        SecurityContextHolder::clear_context();
        let mut filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        filter.set_principal_extractor(Arc::new(|certificate: &X509Certificate| {
            Ok(format!("custom:{}", certificate.subject_dn()))
        }));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Alice,O=Example"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        let authentication = SecurityContextHolder::get_context()
            .get_authentication()
            .expect("authentication should be present");
        assert_eq!(authentication.name(), "custom:CN=Alice,O=Example");
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn skips_authentication_when_already_authenticated() {
        SecurityContextHolder::clear_context();
        let current = PreAuthenticatedAuthenticationToken::with_authorities(
            Arc::new("existing".to_owned()),
            None,
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );
        let context = SecurityContextHolder::create_empty_context();
        context.set_authentication(Some(Arc::new(current)));
        SecurityContextHolder::set_context(context);

        let filter = X509AuthenticationFilter::new(Arc::new(FailingAuthenticationManager));
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Alice,O=Example"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should not authenticate again");

        assert_eq!(
            SecurityContextHolder::get_context()
                .get_authentication()
                .expect("authentication should be preserved")
                .name(),
            "existing"
        );
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn reauthenticates_when_principal_changes() {
        SecurityContextHolder::clear_context();
        let current = PreAuthenticatedAuthenticationToken::with_authorities(
            Arc::new("existing".to_owned()),
            None,
            AuthorityUtils::create_authority_list(["ROLE_USER"]),
        );
        let context = SecurityContextHolder::create_empty_context();
        context.set_authentication(Some(Arc::new(current)));
        SecurityContextHolder::set_context(context);

        let mut filter = X509AuthenticationFilter::new(Arc::new(MockAuthenticationManager));
        filter.set_check_for_principal_changes(true);
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Alice,O=Example"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert_eq!(
            SecurityContextHolder::get_context()
                .get_authentication()
                .expect("authentication should be replaced")
                .name(),
            "Alice"
        );
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn propagates_failure_when_configured_to_stop() {
        SecurityContextHolder::clear_context();
        let mut filter = X509AuthenticationFilter::new(Arc::new(FailingAuthenticationManager));
        filter.set_continue_filter_chain_on_unsuccessful_authentication(false);
        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Alice,O=Example"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        let result = filter.do_filter(&mut request, &mut response, &chain).await;

        assert!(result.is_err());
        SecurityContextHolder::clear_context();
    }

    #[tokio::test]
    async fn applies_authentication_details_source() {
        SecurityContextHolder::clear_context();
        let captured = Arc::new(Mutex::new(None));
        let manager = Arc::new(CapturingAuthenticationManager {
            details: captured.clone(),
        });
        let mut filter = X509AuthenticationFilter::new(manager);
        filter.set_authentication_details_source(Arc::new(MarkerDetailsSource));

        let mut request = test_request();
        request.set_attribute(
            DEFAULT_CLIENT_CERTIFICATE_ATTRIBUTE,
            AnyValue::from("CN=Alice,O=Example"),
        );
        let mut response = test_response();
        let chain = RecordingChain::default();

        filter
            .do_filter(&mut request, &mut response, &chain)
            .await
            .expect("filter should succeed");

        assert_eq!(*captured.lock().await, Some("marker-details".to_string()));
        SecurityContextHolder::clear_context();
    }
}
