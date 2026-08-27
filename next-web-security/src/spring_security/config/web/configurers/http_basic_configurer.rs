use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, LazyLock},
};

use next_web_core::{
    http::{MediaType, StatusCode},
    traits::required::Required,
    web::accept::{ContentNegotiationStrategy, HeaderContentNegotiationStrategy},
};

use crate::{
    authorization::{AuthenticationDetailsSource, AuthenticationManager},
    config::{
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        security_configurer_adapter::SecurityConfigurerAdapter,
        web::{
            configurers::{BaseHttpConfigurer, ErrorHandlingConfigurer, LogoutConfigurer},
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::authority::FactorGrantedAuthority,
    web::{
        authentication::{
            logout::HttpStatusReturningLogoutSuccessHandler, BasicAuthenticationEntryPoint,
            BasicAuthenticationFilter, DelegatingAuthenticationEntryPoint, HttpStatusEntryPoint,
            RememberMeServices,
        },
        context::SecurityContextRepository,
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{
            AndRequestMatcher, MediaTypeRequestMatcher, NegatedRequestMatcher, OrRequestMatcher,
            RequestHeaderRequestMatcher, RequestMatcher,
        },
        AuthenticationEntryPoint,
    },
};

/// Request header matcher for X-Requested-With: XMLHttpRequest.
static X_REQUESTED_WITH: LazyLock<Arc<dyn RequestMatcher>> = LazyLock::new(|| {
    Arc::new(RequestHeaderRequestMatcher::new(
        "X-Requested-With",
        Some("XMLHttpRequest".into()),
    ))
});

/// Adds HTTP basic based authentication. All attributes have reasonable defaults making
/// all parameters are optional.
///
/// # Security Filters
///
/// The following Filters are populated:
///
/// * `BasicAuthenticationFilter`
///
/// # Shared Objects Created
///
/// * `AuthenticationEntryPoint` - populated with the `authentication_entry_point`
///   (default `BasicAuthenticationEntryPoint`)
///
/// # Shared Objects Used
///
/// The following shared objects are used:
///
/// * `AuthenticationManager`
/// * `RememberMeServices`
#[derive(Clone)]
pub struct HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    authentication_details_source: Option<Arc<dyn AuthenticationDetailsSource>>,
    basic_auth_entry_point: BasicAuthenticationEntryPoint,
    security_context_repository: Option<Arc<dyn SecurityContextRepository>>,

    base: BaseHttpConfigurer<Self, B>,
}

impl<B> HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    const DEFAULT_REALM: &str = "Realm";

    /// Allows easily changing the realm, but leaving the remaining defaults in place. If
    /// `authentication_entry_point` has been invoked, invoking this method will result in
    /// an error.
    ///
    /// # Arguments
    ///
    /// * `realm_name` - The HTTP Basic realm to use.
    pub fn realm_name(&mut self, realm_name: impl Into<String>) -> &mut Self {
        self.basic_auth_entry_point.set_realm_name(realm_name);
        self.basic_auth_entry_point.after_properties_set();
        self
    }

    /// The `AuthenticationEntryPoint` to be populated on `BasicAuthenticationFilter` in
    /// the event that authentication fails. The default uses
    /// `BasicAuthenticationEntryPoint` with the realm "Realm".
    ///
    /// # Arguments
    ///
    /// * `authentication_entry_point` - The `AuthenticationEntryPoint` to use.
    pub fn authentication_entry_point(
        &mut self,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> &mut Self {
        self.authentication_entry_point = Some(authentication_entry_point);
        self
    }

    /// Specifies a custom `AuthenticationDetailsSource` to use for basic authentication.
    /// The default is `WebAuthenticationDetailsSource`.
    ///
    /// # Arguments
    ///
    /// * `authentication_details_source` - The custom `AuthenticationDetailsSource` to
    ///   use.
    pub fn authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) -> &mut Self {
        self.authentication_details_source = Some(authentication_details_source);
        self
    }

    /// Specifies a custom `SecurityContextRepository` to use for basic authentication.
    /// The default is `RequestAttributeSecurityContextRepository`.
    ///
    /// # Arguments
    ///
    /// * `security_context_repository` - The custom `SecurityContextRepository` to use.
    pub fn security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) -> &mut Self {
        self.security_context_repository = Some(security_context_repository);
        self
    }

    /// Registers default entry points and logout success handlers.
    fn register_defaults(&self, http: &mut B)
    where
        B: 'static,
    {
        let content_negotiation_strategy = http
            .shared_object::<Arc<dyn ContentNegotiationStrategy>>()
            .map(Clone::clone)
            .unwrap_or_else(|| Arc::new(HeaderContentNegotiationStrategy::default()));

        let mut rest_matcher = MediaTypeRequestMatcher::with_strategy(
            content_negotiation_strategy.clone(),
            vec![
                MediaType::application_atom_xml(),
                MediaType::application_form_urlencoded(),
                MediaType::application_json(),
                MediaType::application_octet_stream(),
                MediaType::application_xml(),
                MediaType::multipart_form_data(),
                MediaType::text_xml(),
            ],
        );
        rest_matcher.set_ignored_media_types(vec![MediaType::all()]);

        let mut all_matcher = MediaTypeRequestMatcher::with_strategy(
            content_negotiation_strategy.clone(),
            vec![MediaType::all()],
        );
        all_matcher.set_use_equals(true);

        let not_html_matcher = Arc::new(NegatedRequestMatcher::new(
            MediaTypeRequestMatcher::with_strategy(
                content_negotiation_strategy,
                vec![MediaType::text_html()],
            ),
        ));

        let rest_not_html_matcher = Arc::new(AndRequestMatcher::new(vec![
            not_html_matcher,
            Arc::new(rest_matcher),
        ]));

        let preferred_matcher = Arc::new(OrRequestMatcher::new(vec![
            X_REQUESTED_WITH.clone(),
            rest_not_html_matcher,
            Arc::new(all_matcher),
        ]));

        self.register_default_entry_point(http, preferred_matcher.clone());
        self.register_default_logout_success_handler(http, preferred_matcher);
    }

    /// Registers the default authentication entry point for the given matcher.
    fn register_default_entry_point(&self, http: &mut B, preferred_matcher: Arc<dyn RequestMatcher>)
    where
        B: 'static,
    {
        let error_handling = match http.configurer_mut::<ErrorHandlingConfigurer<B>>() {
            Some(eh) => eh,
            None => return,
        };

        let entry_point = self
            .authentication_entry_point
            .as_ref()
            .map(Clone::clone)
            .expect("authentication_entry_point must be set");

        error_handling
            .default_authentication_entry_point_for(entry_point.clone(), preferred_matcher.clone());
        error_handling.default_denied_handler_for_missing_authority_with_builder(
            move |ep| {
                ep.add_entry_point_for(entry_point, preferred_matcher);
            },
            FactorGrantedAuthority::PASSWORD_AUTHORITY,
        );
    }

    /// Registers the default logout success handler for the given matcher.
    fn register_default_logout_success_handler(
        &self,
        http: &mut B,
        preferred_matcher: Arc<dyn RequestMatcher>,
    ) where
        B: 'static,
    {
        let logout_configurer = match http.configurer_mut::<LogoutConfigurer<B>>() {
            Some(logout_configurer) => logout_configurer,
            None => return,
        };

        let handler = HttpStatusReturningLogoutSuccessHandler::new(StatusCode::NO_CONTENT);
        logout_configurer.default_logout_success_handler_for(handler, preferred_matcher);
    }
}

impl<B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_object()
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        self.base.get_mut_object()
    }
}

impl<B> SecurityConfigurer<DefaultSecurityFilterChain, B> for HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
    B: 'static,
{
    fn init(&mut self, http: &mut B) {
        self.register_defaults(http);
    }

    fn configure(&mut self, http: &mut B) {
        let authentication_manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .map(Clone::clone)
            .expect("AuthenticationManager must be available");

        let entry_point = self
            .authentication_entry_point
            .take()
            .expect("authentication_entry_point must be set");

        let mut basic_authentication_filter =
            BasicAuthenticationFilter::new(authentication_manager, entry_point);

        self.authentication_details_source
            .take()
            .map(|authentication_details_source| {
                basic_authentication_filter
                    .set_authentication_details_source(authentication_details_source)
            });

        self.security_context_repository
            .take()
            .map(|security_context_repository| {
                basic_authentication_filter
                    .set_security_context_repository(security_context_repository)
            });

        http.shared_object::<Arc<dyn RememberMeServices>>()
            .map(Clone::clone)
            .map(|remember_me_services| {
                basic_authentication_filter.set_remember_me_services(remember_me_services)
            });

        basic_authentication_filter.set_security_context_holder_strategy(
            self.base.get_security_context_holder_strategy().clone(),
        );

        http.add_filter(basic_authentication_filter);
    }
}

impl<B> Deref for HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    type Target = BaseHttpConfigurer<Self, B>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<B> DerefMut for HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl<B> Default for HttpBasicConfigurer<B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        let mut http_basic_configurer = Self {
            authentication_entry_point: None,
            authentication_details_source: None,
            basic_auth_entry_point: Default::default(),
            security_context_repository: None,

            base: Default::default(),
        };
        http_basic_configurer.realm_name(Self::DEFAULT_REALM);

        let authentication_entry_point = DelegatingAuthenticationEntryPoint::builder()
            .add_entry_point_for(
                Arc::new(HttpStatusEntryPoint::new(StatusCode::UNAUTHORIZED)),
                X_REQUESTED_WITH.clone(),
            )
            .default_entry_point(Arc::new(
                http_basic_configurer.basic_auth_entry_point.clone(),
            ))
            .build();
        http_basic_configurer.authentication_entry_point = Some(authentication_entry_point);

        http_basic_configurer
    }
}
