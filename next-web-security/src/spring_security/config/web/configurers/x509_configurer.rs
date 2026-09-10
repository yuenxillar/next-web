use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::required::Required;

use crate::{
    authentication::authentication_details_source::AuthenticationDetailsSource,
    authorization::AuthenticationManager,
    config::{
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{BaseHttpConfigurer, ErrorHandlingConfigurer},
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::{
        authority::{AuthorityUtils, FactorGrantedAuthority},
        userdetails::{
            AuthenticationUserDetailsService, UserDetailsByNameServiceWrapper, UserDetailsService,
        },
    },
    web::{
        authentication::{
            preauth::{
                x509::{X509AuthenticationFilter, X509PrincipalExtractor},
                PreAuthenticatedAuthenticationProvider, PreAuthenticatedAuthenticationToken,
            },
            Http403ForbiddenEntryPoint,
        },
        context::RequestAttributeSecurityContextRepository,
        default_security_filter_chain::DefaultSecurityFilterChain,
        AuthenticationEntryPoint,
    },
};

/// Adds X.509 based pre-authentication to an application.
///
/// Since validating the certificate happens when the client connects, the requesting and
/// validation of the client certificate should be performed by the container. The
/// framework then uses the certificate to look up the authentication for the user.
///
/// # Security Filters
///
/// The following filters are populated:
///
/// * `X509AuthenticationFilter`
///
/// # Shared Objects Created
///
/// * `AuthenticationEntryPoint` - populated with an `Http403ForbiddenEntryPoint`
/// * A `PreAuthenticatedAuthenticationProvider` is registered with the authentication
///   manager builder
///
/// # Shared Objects Used
///
/// * A `UserDetailsService` shared object is used when no
///   `AuthenticationUserDetailsService` is specified
#[derive(Clone)]
pub struct X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    x509_authentication_filter: Option<X509AuthenticationFilter>,
    x509_principal_extractor: Option<Arc<dyn X509PrincipalExtractor>>,
    authentication_user_details_service:
        Option<Arc<dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>>>,
    authentication_details_source: Option<Arc<dyn AuthenticationDetailsSource>>,

    base: BaseHttpConfigurer<Self, H>,
}

impl<H> X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    /// Allows specifying the entire [`X509AuthenticationFilter`]. If this is specified,
    /// the properties on this configurer will not be populated on the filter.
    ///
    /// # Arguments
    ///
    /// * `x509_authentication_filter` - the filter to use
    pub fn x509_authentication_filter(
        &mut self,
        x509_authentication_filter: X509AuthenticationFilter,
    ) -> &mut Self {
        self.x509_authentication_filter = Some(x509_authentication_filter);
        self
    }

    /// Specifies the [`X509PrincipalExtractor`].
    ///
    /// # Arguments
    ///
    /// * `x509_principal_extractor` - the extractor to use
    pub fn x509_principal_extractor<T>(&mut self, x509_principal_extractor: T) -> &mut Self
    where
        T: X509PrincipalExtractor,
        T: 'static,
    {
        self.x509_principal_extractor = Some(Arc::new(x509_principal_extractor));
        self
    }

    /// Specifies the `AuthenticationUserDetailsService` to use. If not specified, then
    /// the `UserDetailsService` shared object will be used by default.
    ///
    /// # Arguments
    ///
    /// * `authentication_user_details_service` - the service to use
    pub fn authentication_user_details_service(
        &mut self,
        authentication_user_details_service: Arc<
            dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>,
        >,
    ) -> &mut Self {
        self.authentication_user_details_service = Some(authentication_user_details_service);
        self
    }

    /// Shortcut for invoking [`Self::authentication_user_details_service`] with a
    /// [`UserDetailsByNameServiceWrapper`].
    ///
    /// # Arguments
    ///
    /// * `user_details_service` - the service to use
    pub fn user_details_service<T>(&mut self, user_details_service: T) -> &mut Self
    where
        T: UserDetailsService,
        T: 'static,
    {
        self.authentication_user_details_service(Arc::new(UserDetailsByNameServiceWrapper::<
            PreAuthenticatedAuthenticationToken,
        >::new(Arc::new(
            user_details_service,
        ))))
    }

    /// Specifies the `AuthenticationDetailsSource` to use.
    ///
    /// # Arguments
    ///
    /// * `authentication_details_source` - the details source to use
    pub fn authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) -> &mut Self {
        self.authentication_details_source = Some(authentication_details_source);
        self
    }

    /// Specifies the regex to extract the principal from the certificate. If not
    /// specified, the default expression from [`SubjectDnX509PrincipalExtractor`] is
    /// used.
    ///
    /// # Arguments
    ///
    /// * `subject_principal_regex` - the regex to extract the user principal from the
    ///   certificate (i.e. `CN=(.*?)(?:,|$)`)
    #[allow(deprecated)]
    pub fn subject_principal_regex(&mut self, subject_principal_regex: &str) -> &mut Self {
        let mut principal_extractor =
            crate::web::authentication::preauth::x509::SubjectDnX509PrincipalExtractor::new();
        principal_extractor.set_subject_dn_regex(subject_principal_regex);
        self.x509_principal_extractor = Some(Arc::new(principal_extractor));
        self
    }

    fn get_authentication_user_details_service(
        &mut self,
        http: &H,
    ) -> Arc<dyn AuthenticationUserDetailsService<PreAuthenticatedAuthenticationToken>> {
        if let Some(authentication_user_details_service) =
            self.authentication_user_details_service.as_ref()
        {
            return authentication_user_details_service.clone();
        }

        let user_details_service = http
            .shared_object::<Arc<dyn UserDetailsService>>()
            .cloned()
            .expect(
                "A UserDetailsService is required to enable x509(). Configure one via \
                 user_details_service or register a shared UserDetailsService.",
            );

        let service = Arc::new(UserDetailsByNameServiceWrapper::<
            PreAuthenticatedAuthenticationToken,
        >::new(user_details_service));
        self.authentication_user_details_service = Some(service.to_owned());

        service
    }

    fn get_filter(&mut self, http: &H) -> X509AuthenticationFilter {
        if let Some(filter) = self.x509_authentication_filter.as_ref() {
            return filter.clone();
        }

        let authentication_manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .cloned()
            .expect("AuthenticationManager must be available");

        let mut filter = X509AuthenticationFilter::new(authentication_manager);
        if let Some(principal_extractor) = self.x509_principal_extractor.take() {
            filter.set_principal_extractor(principal_extractor);
        }
        if let Some(authentication_details_source) = self.authentication_details_source.take() {
            filter.set_authentication_details_source(authentication_details_source);
        }

        filter.set_security_context_repository(Arc::new(
            RequestAttributeSecurityContextRepository::default(),
        ));
        filter.set_security_context_holder_strategy(
            self.base.get_security_context_holder_strategy().to_owned(),
        );
        self.x509_authentication_filter = Some(filter.clone());

        filter
    }
}

impl<H> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, H>> for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, H> {
        self.base.get_mut_object()
    }
}

impl<H> SecurityConfigurer<DefaultSecurityFilterChain, H> for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
    H: 'static,
{
    fn init(&mut self, http: &mut H) {
        let authentication_user_details_service =
            self.get_authentication_user_details_service(http);
        let mut authentication_provider = PreAuthenticatedAuthenticationProvider::default();
        authentication_provider
            .set_pre_authenticated_user_details_service(authentication_user_details_service);
        authentication_provider.set_granted_authorities(AuthorityUtils::create_authority_list([
            FactorGrantedAuthority::X509_AUTHORITY.to_string(),
        ]));

        http.authentication_provider(Arc::new(authentication_provider));

        let forbidden: Arc<dyn AuthenticationEntryPoint> =
            Arc::new(Http403ForbiddenEntryPoint::default());
        http.set_shared_object::<Arc<dyn AuthenticationEntryPoint>>(forbidden.clone());

        if let Some(error_handling_configurer) = http.configurer_mut::<ErrorHandlingConfigurer<H>>()
        {
            error_handling_configurer.default_denied_handler_for_missing_authority_with_builder(
                |ep| {
                    ep.default_entry_point(forbidden);
                },
                FactorGrantedAuthority::X509_AUTHORITY,
            );
        }
    }

    fn configure(&mut self, http: &mut H) {
        let filter = self.get_filter(http);
        http.add_filter(filter);
    }
}

impl<H> Deref for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    type Target = BaseHttpConfigurer<Self, H>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<H> DerefMut for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<H> Default for X509Configurer<H>
where
    H: HttpSecurityBuilder<H>,
{
    fn default() -> Self {
        Self {
            x509_authentication_filter: None,
            x509_principal_extractor: None,
            authentication_user_details_service: None,
            authentication_details_source: None,
            base: Default::default(),
        }
    }
}
