use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

use next_web_context::ApplicationContextExt;
use next_web_core::{ApplicationContext, Required};

use crate::{
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{configurers::BaseHttpConfigurer, http_security_builder::HttpSecurityBuilder},
    },
    saml2::registration::RelyingPartyRegistrationRepository,
    web::{
        authentication::BasicAuthenticationFilter,
        default_security_filter_chain::DefaultSecurityFilterChain,
        saml2::metadata::Saml2MetadataFilter,
        util::matcher::PathPatternRequestMatcher,
    },
};

/// Configures SAML 2.0 metadata endpoint.
///
/// When fully implemented, this will:
/// - Create `Saml2MetadataFilter` serving metadata at configurable endpoints
///   (default: `/saml2/metadata`, `/saml2/metadata/{registrationId}`,
///   `/saml2/service-provider-metadata/{registrationId}`)
/// - Serve per-registration or aggregated `EntityDescriptor` XML
///
/// Requires: `RelyingPartyRegistrationRepository` (from `Saml2LoginConfigurer` or bean).
#[derive(Clone)]
pub struct Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    metadata_url: Option<String>,
    registrations: Option<Arc<dyn RelyingPartyRegistrationRepository>>,
    context: Arc<RwLock<Box<dyn ApplicationContext>>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    /// Creates a configurer that resolves the registrations from the given
    /// context when none is set through the DSL or shared with the builder.
    pub fn new(context: Arc<RwLock<Box<dyn ApplicationContext>>>) -> Self {
        Self {
            context,
            ..Self::default()
        }
    }

    /// Sets the relying party registrations the metadata describes.
    ///
    /// A repository set here takes precedence over the one shared with the
    /// builder and the one contributed to the application context.
    pub fn relying_party_registration_repository(
        &mut self,
        registrations: Arc<dyn RelyingPartyRegistrationRepository>,
    ) -> &mut Self {
        self.registrations = Some(registrations);
        self
    }

    /// Set the metadata endpoint URL. Supports `{registrationId}` placeholder.
    /// Default: `"/saml2/metadata"`.
    ///
    /// # Panics
    ///
    /// Panics when the URL is empty or whitespace.
    pub fn metadata_url(mut self, url: &str) -> Self {
        assert!(!url.trim().is_empty(), "metadataUrl cannot be empty");
        self.metadata_url = Some(url.to_string());
        self
    }

    /// Resolves the registrations the metadata describes.
    ///
    /// The registrations set through the DSL win, then the ones shared with the
    /// builder, then the ones the application context contributes.
    fn resolve_registrations(&mut self, http: &H) -> Option<Arc<dyn RelyingPartyRegistrationRepository>> {
        if let Some(registrations) = &self.registrations {
            return Some(registrations.clone());
        }

        if let Some(registrations) =
            http.shared_object::<Arc<dyn RelyingPartyRegistrationRepository>>()
        {
            return Some(registrations.clone());
        }

        let context = self
            .context
            .write()
            .expect("the application context lock is poisoned");

        (*context)
            .get_singletons_by_type::<Arc<dyn RelyingPartyRegistrationRepository>>()
            .first()
            .map(|registrations| (*registrations).clone())
    }
}

impl<H> Default for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            metadata_url: None,
            registrations: None,
            context: Arc::new(RwLock::new(
                Box::new(super::super::builders::StandaloneApplicationContext::default())
                    as Box<dyn ApplicationContext>,
            )),
            base: Default::default(),
        }
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>>
    for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn init(&mut self, http: &mut H) {
        let Some(registrations) = self.resolve_registrations(http) else {
            panic!("relyingPartyRegistrationRepository cannot be null");
        };

        self.registrations = Some(registrations);
    }

    fn configure(&mut self, http: &mut H) {
        let Some(registrations) = self.registrations.clone() else {
            // A configurer that was not initialized has nothing to describe, and
            // `init` is where a chain that forgot the repository is reported.
            return;
        };

        let mut filter = Saml2MetadataFilter::new(registrations);
        if let Some(metadata_url) = &self.metadata_url {
            filter.set_request_matcher(Arc::new(PathPatternRequestMatcher::path_pattern(
                None,
                metadata_url,
            )));
        }

        // The metadata endpoint sits where the reference implementation puts it:
        // ahead of the basic authentication filter.
        http.add_filter_before::<Saml2MetadataFilter, BasicAuthenticationFilter>(filter);
    }
}

impl<H> Deref for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for Saml2MetadataConfigurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::{body::Body, extract::Request, response::Response};
    use next_web_core::{
        async_trait,
        http::filter::{FilterError, HttpFilter, HttpFilterChain},
    };

    use super::*;
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            web::builders::{HttpSecurity, StandaloneApplicationContext},
        },
        saml2::registration::{
            InMemoryRelyingPartyRegistrationRepository, RelyingPartyRegistration,
        },
        web::security_filter_chain::SecurityFilterChain,
    };

    /// Returns a context that contributes no relying party registrations.
    fn context() -> Arc<RwLock<Box<dyn ApplicationContext>>> {
        Arc::new(RwLock::new(
            Box::new(StandaloneApplicationContext::default()) as Box<dyn ApplicationContext>
        ))
    }

    /// Returns an HTTP security a configurer can be applied to.
    fn http_security() -> HttpSecurity {
        HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new())
    }

    /// Returns a repository over one registration.
    fn repository() -> Arc<InMemoryRelyingPartyRegistrationRepository> {
        Arc::new(InMemoryRelyingPartyRegistrationRepository::new(vec![
            RelyingPartyRegistration::with_registration_id("okta").build(),
        ]))
    }

    /// Returns a `GET` request for an absolute URL.
    fn request(url: &str) -> Request {
        Request::builder()
            .method("GET")
            .uri(url)
            .body(Body::empty())
            .expect("a valid request")
    }

    /// Records whether the rest of the chain ran.
    #[derive(Clone, Default)]
    struct RecordingChain {
        ran: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl HttpFilterChain for RecordingChain {
        async fn do_filter(
            &self,
            _request: &mut dyn next_web_core::http::HttpRequest,
            _response: &mut dyn next_web_core::http::HttpResponse,
        ) -> Result<(), FilterError> {
            self.ran
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    /// Returns the metadata filter the configurer registered on the chain.
    fn metadata_filter(http: &mut HttpSecurity) -> Arc<dyn HttpFilter> {
        http.build()
            .get_filters()
            .iter()
            .find(|filter| filter.name() == "Saml2MetadataFilter")
            .cloned()
            .expect("the configurer registered the metadata filter")
    }

    #[test]
    fn the_metadata_filter_is_registered() {
        let mut http = http_security();
        let mut configurer = Saml2MetadataConfigurer::new(context());
        configurer.relying_party_registration_repository(repository());

        configurer.init(&mut http);
        configurer.configure(&mut http);

        let filter = metadata_filter(&mut http);
        assert_eq!(filter.name(), "Saml2MetadataFilter");
    }

    #[test]
    #[should_panic(expected = "relyingPartyRegistrationRepository cannot be null")]
    fn a_missing_repository_is_reported() {
        let mut http = http_security();
        let mut configurer = Saml2MetadataConfigurer::new(context());

        configurer.init(&mut http);
    }

    #[tokio::test]
    async fn a_custom_metadata_url_replaces_the_endpoints() {
        let mut http = http_security();
        let mut configurer =
            Saml2MetadataConfigurer::new(context()).metadata_url("/saml/metadata");
        configurer.relying_party_registration_repository(repository());
        configurer.init(&mut http);
        configurer.configure(&mut http);
        let filter = metadata_filter(&mut http);

        let mut response = Response::default();
        let chain = RecordingChain::default();
        filter
            .do_filter(
                &mut request("http://sp.example.org/saml2/metadata/okta"),
                &mut response,
                &chain,
            )
            .await
            .expect("the filter runs");
        assert!(
            chain.ran.load(std::sync::atomic::Ordering::SeqCst),
            "the default endpoint is not served anymore"
        );

        let mut response = Response::default();
        filter
            .do_filter(
                &mut request("http://sp.example.org/saml/metadata/okta"),
                &mut response,
                &RecordingChain::default(),
            )
            .await
            .expect("the filter runs");
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn the_repository_is_taken_from_the_context() {
        let mut application_context = StandaloneApplicationContext::default();
        let registrations: Arc<dyn RelyingPartyRegistrationRepository> = repository();
        application_context.insert_singleton_with_name(registrations, "registrations");
        let context: Arc<RwLock<Box<dyn ApplicationContext>>> =
            Arc::new(RwLock::new(Box::new(application_context)));

        let mut http = http_security();
        let mut configurer = Saml2MetadataConfigurer::new(context);
        configurer.init(&mut http);
        configurer.configure(&mut http);
        let filter = metadata_filter(&mut http);

        let mut response = Response::default();
        filter
            .do_filter(
                &mut request("http://sp.example.org/saml2/metadata/okta"),
                &mut response,
                &RecordingChain::default(),
            )
            .await
            .expect("the filter runs");

        assert_eq!(response.status(), 200);
    }
}
