use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{atomic::Ordering, Arc, RwLock},
};

use next_web_core::{
    any::AnyClone,
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    ApplicationContext, BoxAny, Named, Ordered,
};

use next_web_context::ApplicationEventPublisher;

use crate::{
    access::{
        hierarchicalroles::{NullRoleHierarchy, RoleHierarchy},
        intercept::RequestAuthorizationContext,
    },
    authentication::AuthenticationProvider,
    authorization::{
        AuthenticationManager, AuthorizationEventPublisher, AuthorizationManagerFactory,
        AuthorizationResult, DefaultAuthorizationManagerFactory, NextAuthorizationEventPublisher,
    },
    config::{
        authentication::{
            builders::authentication_manager_builder::AuthenticationManagerBuilder,
            provider_manager_builder::ProviderManagerBuilder,
        },
        base_configured_security_builder::{
            BaseConfiguredSecurityBuilder, BaseConfiguredSecurityBuilderExt, BuildState,
        },
        http::core::granted_authority_defaults::GrantedAuthorityDefaults,
        security_builder::SecurityBuilder,
        security_configurer::SecurityConfigurer,
        web::{
            base_request_matcher_registry::BaseRequestMatcherRegistry,
            builders::filter_order_registration::FilterOrderRegistration,
            configurers::{
                AnonymousConfigurer, AuthorizationManagerRequestMatcherRegistry,
                AuthorizeHttpRequestsConfigurer, CorsConfigurer, CsrfConfigurer,
                ErrorHandlingConfigurer, FormLoginConfigurer, HeadersConfigurer,
                HttpBasicConfigurer, HttpsRedirectConfigurer, LogoutConfigurer,
                OneTimeTokenLoginConfigurer, PasswordManagementConfigurer, PortMapperConfigurer,
                RememberMeConfigurer, RequestCacheConfigurer, Saml2LoginConfigurer,
                Saml2LogoutConfigurer, Saml2MetadataConfigurer, SecurityContextConfigurer,
                SessionManagementConfigurer, WebAuthnConfigurer, X509Configurer,
            },
            http_security_builder::HttpSecurityBuilder,
        },
    },
    core::{userdetails::UserDetailsService, Authentication},
    web::{
        default_security_filter_chain::DefaultSecurityFilterChain,
        util::matcher::{
            AnyRequestMatcher, OrRequestMatcher, PathPatternRequestMatcherBuilder, RequestMatcher,
        },
    },
};

#[cfg(all(feature = "oauth2-client", feature = "oauth2-resource-server"))]
use crate::config::web::configurers::oauth2::client::OidcLogoutConfigurer;
#[cfg(feature = "oauth2-authorization-server")]
use crate::config::web::configurers::OAuth2AuthorizationServerConfigurer;
#[cfg(feature = "oauth2-resource-server")]
use crate::config::web::configurers::OAuth2ResourceServerConfigurer;
#[cfg(feature = "oauth2-client")]
use crate::config::web::configurers::{OAuth2ClientConfigurer, OAuth2LoginConfigurer};

/// Configures the web security of one HTTP filter chain.
///
/// The instance collects the configurers that contribute the filters of a
/// single [`DefaultSecurityFilterChain`]. [`SecurityBuilder::build`] then orders
/// those filters, wraps them with the request matcher of this instance and
/// produces the chain.
///
/// By default the chain matches every request, and no request is rejected until
/// authorization rules are declared with [`Self::authorize_http_requests`]. The
/// matcher is narrowed with [`Self::security_matcher`],
/// [`Self::security_matcher_with_strs`] or [`Self::security_matchers`], which is
/// how one application declares several independent chains — one for its API,
/// one for its login pages, and so on.
///
/// # Example
///
/// ```ignore
/// let mut http = HttpSecurity::new(authentication_manager_builder, shared_objects);
///
/// http.authorize_http_requests(|authorize| {
///         authorize.request_matchers(vec!["/admin/**"]).has_role("ADMIN");
///         authorize.request_matchers(vec!["/**"]).has_role("USER");
///     })
///     .http_basic(|basic| {
///         basic.realm_name("Realm");
///     });
///
/// let chain = http.build();
/// ```
///
/// # Reusing configurers
///
/// Every DSL method looks the configurer type up before applying the closure: a
/// configurer that is already registered is extended, and a configurer is
/// created only the first time its type is seen. Calling a method twice
/// therefore grows the same configurer instead of contributing the same filter
/// twice. A filter takes its position from the filter registration rather than
/// from the order of the calls, so the order in which the DSL methods are
/// invoked does not change the resulting chain.
///
/// # Shared objects
///
/// The builder shares the authentication builder, the application context and
/// the request matcher builder it was created with. Configurers read the objects
/// they need from there, so an object that was not shared makes a configurer
/// fall back to its own default rather than fail.
///
/// # Status of the methods
///
/// Every method states the state of its wiring on the first line of its
/// documentation, so the state of a feature can be read without opening the
/// configurer it registers:
///
/// - `Ok` — the basic flow already works.
/// - `Unverified` — the wiring is in place, but no end-to-end check backs it yet.
/// - `Pending` — the feature cannot be used yet; the documentation of the method
///   names what is missing.
pub struct HttpSecurity {
    request_matcher_configurer: RequestMatcherConfigurer,
    filters: Vec<OrderedFilter>,
    request_matcher: Arc<dyn RequestMatcher>,
    filter_orders: FilterOrderRegistration,
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,

    base: BaseConfiguredSecurityBuilder<DefaultSecurityFilterChain, Self>,
}

impl HttpSecurity {
    /// Creates a builder for one HTTP filter chain.
    ///
    /// The `authentication_builder` is registered as a shared object, because the
    /// authentication manager of the chain is built from it during the build
    /// phase unless [`Self::authentication_manager`] replaces it. Every entry of
    /// `shared_objects` is registered as a shared object as well, and the
    /// [`PathPatternRequestMatcherBuilder`] among them becomes the builder this
    /// instance creates request matchers with.
    pub fn new(
        authentication_builder: AuthenticationManagerBuilder,
        shared_objects: HashMap<TypeId, Box<dyn AnyClone>>,
    ) -> Self {
        let builder = shared_objects
            .get(&TypeId::of::<PathPatternRequestMatcherBuilder>())
            // `Box<dyn AnyClone>` satisfies the blanket `AnyClone` impl as well,
            // so the box has to be dereferenced before the erased value is
            // reached.
            .and_then(|s| {
                (**s)
                    .as_any()
                    .downcast_ref::<PathPatternRequestMatcherBuilder>()
            })
            .cloned()
            .unwrap_or_default();

        let mut http = Self {
            filter_orders: Default::default(),
            request_matcher: Arc::new(AnyRequestMatcher),
            filters: Vec::new(),
            request_matcher_configurer: RequestMatcherConfigurer::new(builder),
            authentication_manager: None,

            base: Default::default(),
        };

        http.base.set_shared_object(authentication_builder);
        http.base.shared_objects.extend(shared_objects.into_iter());

        http
    }

    /// Returns the shared application context of the security builder.
    ///
    /// The context is shared as `Arc<RwLock<Box<dyn ApplicationContext>>>`, so
    /// that a configurer can keep a handle on it while the application keeps
    /// owning the context itself. A builder that was not given one uses an
    /// empty context, so a feature that only may take a value from the context
    /// falls back to its own default instead of failing.
    fn get_context(&mut self) -> Arc<RwLock<Box<dyn ApplicationContext>>> {
        self.base
            .get_shared_object::<Arc<RwLock<Box<dyn ApplicationContext>>>>()
            .cloned()
            .unwrap_or_else(|| {
                tracing::debug!(
                    "No application context was shared with the security builder, so an empty \
                     context is used"
                );
                Arc::new(RwLock::new(
                    Box::new(super::StandaloneApplicationContext::default())
                        as Box<dyn ApplicationContext>,
                ))
            })
    }

    /// Applies `f` to the configurer of type `C`, registering `configurer` first
    /// when no configurer of that type is registered yet.
    ///
    /// The lookup is what makes the DSL methods idempotent: a second call extends
    /// the configurer the first call registered instead of adding a duplicate.
    fn get_or_apply<C, F>(&mut self, configurer: C, mut f: F)
    where
        C: AnyClone + Clone + 'static,
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        F: FnMut(&mut C),
    {
        if let Some(existing_config) = self.configurer_mut::<C>() {
            f(existing_config);
        } else {
            self.with(configurer, f);
        }
    }

    /// Registers `filter` at the order the filter `F1` was registered with, plus
    /// `offset`.
    ///
    /// The order is recorded for the type of `filter` as well, so a later
    /// `add_filter` call for that type reuses the position this call chose.
    ///
    /// # Panics
    ///
    /// Panics when no order is registered for `F1`, which means the filter is not
    /// part of the chain the registration knows about and there is no position to
    /// anchor the new filter to.
    fn add_filter_at_offset<F, F1>(&mut self, filter: F, offset: i32)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        let name = std::any::type_name::<F1>();
        let registered_filter_order = self
            .filter_orders
            .get_order_by_name(name)
            .unwrap_or_else(|| panic!("Filter {} has no registered order", name));
        let order = registered_filter_order + offset;
        self.filters
            .push(OrderedFilter::new(Arc::new(filter), order));
        self.filter_orders.put::<F>(order);
    }

    /// Returns the factory the authorization managers of the authorization
    /// rules are created with.
    ///
    /// The factory of the application is used when it shared one, and a factory
    /// built from the role hierarchy and the granted authority defaults of the
    /// application is used otherwise.
    fn authorization_manager_factory(
        &self,
    ) -> Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>> {
        if let Some(factory) = self
            .get_shared_object::<Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>>(
            )
        {
            return factory.clone();
        }

        let role_hierarchy = self
            .get_shared_object::<Arc<dyn RoleHierarchy>>()
            .cloned()
            .unwrap_or_else(|| Arc::new(NullRoleHierarchy::default()));

        let role_prefix = self
            .get_shared_object::<GrantedAuthorityDefaults>()
            .map(|defaults| defaults.role_prefix().to_owned())
            .unwrap_or_else(|| DEFAULT_ROLE_PREFIX.to_owned());

        let mut factory = DefaultAuthorizationManagerFactory::default();
        factory.set_role_hierarchy(role_hierarchy);
        factory.set_role_prefix(role_prefix.as_str());

        Arc::new(factory)
    }

    /// Returns the builder the request matchers of the authorization rules are
    /// created with.
    fn authorization_request_matcher_builder(&self) -> PathPatternRequestMatcherBuilder {
        self.get_shared_object::<PathPatternRequestMatcherBuilder>()
            .cloned()
            .unwrap_or_default()
    }

    /// Returns the publisher the authorization events are published to.
    ///
    /// The publisher of the application is used when it shared one, a publisher
    /// that publishes the events of an application event publisher is used next,
    /// and a publisher that drops every event is the fallback.
    fn authorization_event_publisher(&self) -> Arc<dyn AuthorizationEventPublisher> {
        if let Some(publisher) = self.get_shared_object::<Arc<dyn AuthorizationEventPublisher>>() {
            return publisher.clone();
        }

        if let Some(publisher) = self.get_shared_object::<Arc<dyn ApplicationEventPublisher>>() {
            return Arc::new(NextAuthorizationEventPublisher::new(publisher.clone()));
        }

        Arc::new(NullAuthorizationEventPublisher)
    }
}

/// The prefix of the role authority a role hierarchy adds by default.
const DEFAULT_ROLE_PREFIX: &str = "ROLE_";

/// An [`AuthorizationEventPublisher`] that drops every event.
///
/// The publisher is the fallback of an HTTP security the application did not
/// contribute a publisher for, which keeps a configuration that does not use
/// authorization events working.
struct NullAuthorizationEventPublisher;

impl AuthorizationEventPublisher for NullAuthorizationEventPublisher {
    fn publish_authorization_event(
        &self,
        _authentication: Arc<dyn Authentication>,
        _value: BoxAny,
        _result: Option<Arc<dyn AuthorizationResult>>,
    ) {
    }
}

impl HttpSecurity {
    /// Ok
    ///
    /// Adds the security headers to the response.
    ///
    /// The closure receives the [`HeadersConfigurer`], which starts from the
    /// default header set — content type options, cache control, strict transport
    /// security and frame options — so a closure that changes nothing applies
    /// exactly those. The configurer can also be disabled from the closure, which
    /// removes the headers from the chain.
    ///
    /// The headers belong to the configuration an application applies by default,
    /// so a chain gets them without calling this method at all.
    pub fn headers<F>(&mut self, headers: F) -> &mut Self
    where
        F: FnMut(&mut HeadersConfigurer<Self>),
    {
        self.get_or_apply(HeadersConfigurer::default(), headers);

        self
    }

    /// Ok
    ///
    /// Adds CORS (Cross-Origin Resource Sharing) support.
    ///
    /// The filter that is registered is decided when the chain is built: a CORS
    /// filter registered under the name `corsFilter` is used when the application
    /// contributes one, otherwise the filter is built from the
    /// `CorsConfigurationSource` the closure configures.
    ///
    /// The configuration an application applies by default adds CORS as well when
    /// the application contributes a configuration source, so this method is
    /// needed only to configure CORS from a chain that configuration does not
    /// build.
    pub fn cors<F>(&mut self, cors: F) -> &mut Self
    where
        F: FnMut(&mut CorsConfigurer<Self>),
    {
        self.get_or_apply(CorsConfigurer::default(), cors);

        self
    }

    /// Ok
    ///
    /// Configures session management.
    ///
    /// The closure receives the configurer that decides how a session is created
    /// and tracked: the creation policy, the identifier an authenticated session
    /// is fixed with, and the concurrency control that limits how many sessions
    /// one principal may hold at a time.
    ///
    /// Concurrency control relies on session destruction events. An application
    /// that does not publish them never releases an expired session from the
    /// count, so the limit is reached and stays reached.
    ///
    /// Session management belongs to the configuration an application applies by
    /// default.
    pub fn session_management<F>(&mut self, session_management: F) -> &mut Self
    where
        F: FnMut(&mut SessionManagementConfigurer<Self>),
    {
        self.get_or_apply(SessionManagementConfigurer::default(), session_management);

        self
    }

    /// Ok
    ///
    /// Configures the mapper that decides which port a redirect between HTTP and
    /// HTTPS targets.
    ///
    /// The mapper is shared on the builder, so every configurer that redirects
    /// between the two schemes uses it — [`Self::redirect_to_https`] most of all.
    /// Without an explicit mapper the redirect falls back to `PortMapperImpl`,
    /// which pairs the HTTP port 80 with 443 and 8080 with 8443.
    pub fn port_mapper<F>(&mut self, port_mapper: F) -> &mut Self
    where
        F: FnMut(&mut PortMapperConfigurer<Self>),
    {
        self.get_or_apply(PortMapperConfigurer::default(), port_mapper);

        self
    }

    /// Ok
    ///
    /// Configures remember-me authentication.
    ///
    /// The closure configures how a user stays authenticated after the session
    /// ends: through a token in a cookie that is validated on the next request, or
    /// through a token that is stored and looked up. A form submits the remember-me
    /// parameter — `remember-me` by default — to ask for it while authenticating.
    pub fn remember_me<F>(&mut self, remember_me: F) -> &mut Self
    where
        F: FnMut(&mut RememberMeConfigurer<Self>),
    {
        self.get_or_apply(RememberMeConfigurer::default(), remember_me);

        self
    }

    /// Ok
    ///
    /// Adds the authorization rules that decide which requests are servable.
    ///
    /// Rules are evaluated in the order they are declared and the first matching
    /// rule decides, so a broad rule declared before a specific one makes the
    /// specific rule unreachable. Declaring the most specific matcher first is how
    /// exceptions are expressed.
    ///
    /// The registry the closure receives builds its authorization managers with
    /// the authorization manager factory, the request matcher builder and the
    /// authorization event publisher of the application when the application
    /// shared them with this builder.
    ///
    /// # Example
    ///
    /// ```ignore
    /// http.authorize_http_requests(|authorize| {
    ///     authorize.request_matchers(vec!["/admin/**"]).has_role("ADMIN");
    ///     authorize.request_matchers(vec!["/**"]).has_role("USER");
    /// });
    /// ```
    ///
    /// A request to `/admin/users` matches the first rule, every other request
    /// matches the second one.
    pub fn authorize_http_requests<F>(&mut self, mut authorize_http_requests: F) -> &mut Self
    where
        F: FnMut(&mut AuthorizationManagerRequestMatcherRegistry),
    {
        let configurer = AuthorizeHttpRequestsConfigurer::<Self>::with_parts(
            self.authorization_manager_factory(),
            self.authorization_request_matcher_builder(),
            self.authorization_event_publisher(),
        );
        self.get_or_apply(configurer, |configurer| {
            authorize_http_requests(configurer.registry_mut());
        });

        self
    }

    /// Ok
    ///
    /// Configures the request cache.
    ///
    /// The cache stores the request that was rejected because the user was not
    /// authenticated, so the request can be replayed once the user authenticated.
    /// That replay is what makes a redirect to the login page return the user to
    /// the page that was actually requested. The closure replaces the cache or
    /// disables the caching.
    ///
    /// Request caching belongs to the configuration an application applies by
    /// default.
    pub fn request_cache<F>(&mut self, request_cache: F) -> &mut Self
    where
        F: FnMut(&mut RequestCacheConfigurer<Self>),
    {
        self.get_or_apply(RequestCacheConfigurer::default(), request_cache);

        self
    }

    /// Ok
    ///
    /// Configures the handling of the errors the chain raises.
    ///
    /// The configurer turns a failure into a response: the entry point answers a
    /// request that carries no authentication, and the access denied handler
    /// answers a request whose authentication is insufficient. The closure
    /// replaces either, for example to send denied requests to a dedicated page
    /// instead of an error.
    ///
    /// Error handling belongs to the configuration an application applies by
    /// default.
    pub fn error_handling<F>(&mut self, error_handling: F) -> &mut Self
    where
        F: FnMut(&mut ErrorHandlingConfigurer<Self>),
    {
        self.get_or_apply(ErrorHandlingConfigurer::default(), error_handling);

        self
    }

    /// Ok
    ///
    /// Configures how the security context is loaded and saved between requests.
    ///
    /// The context is restored before the chain runs and saved after it returns,
    /// so the authentication one request established is available to the next one.
    /// The closure replaces the repository that performs the load and the save.
    ///
    /// The security context belongs to the configuration an application applies by
    /// default.
    pub fn security_context<F>(&mut self, security_context: F) -> &mut Self
    where
        F: FnMut(&mut SecurityContextConfigurer<Self>),
    {
        self.get_or_apply(SecurityContextConfigurer::default(), security_context);

        self
    }

    /// Ok
    ///
    /// Adds CSRF (Cross-Site Request Forgery) protection.
    ///
    /// The configurer rejects a state changing request that does not carry the
    /// token it issued, which is why a form has to render the token and a script
    /// has to send it. The closure configures where the tokens are stored, or
    /// exempts the requests that cannot carry one.
    ///
    /// Protection belongs to the configuration an application applies by default.
    /// A chain that is authenticated by a bearer token rather than by a cookie
    /// usually disables it, because such a chain has no session a forged request
    /// could ride on.
    pub fn csrf<F>(&mut self, csrf: F) -> &mut Self
    where
        F: FnMut(&mut CsrfConfigurer<Self>),
    {
        let configurer = CsrfConfigurer::default();
        self.get_or_apply(configurer, csrf);

        self
    }

    /// Ok
    ///
    /// Adds logout support.
    ///
    /// A request to the logout URL — `/logout` by default, and a `POST` while CSRF
    /// protection is on — invalidates the session, clears the remember-me
    /// authentication and the security context, and then answers with the success
    /// handling. The closure changes the URL, the handlers and the success
    /// handling.
    ///
    /// Logout belongs to the configuration an application applies by default.
    pub fn logout<F>(&mut self, logout: F) -> &mut Self
    where
        F: FnMut(&mut LogoutConfigurer<Self>),
    {
        self.get_or_apply(LogoutConfigurer::default(), logout);

        self
    }

    /// Ok
    ///
    /// Configures how a request without an authentication is represented.
    ///
    /// An anonymous authentication is written into the context when the chain
    /// established none, so a rule that requires an authority rejects the request
    /// instead of the chain having nothing to evaluate. The closure changes the
    /// authorities of that authentication, or disables the representation
    /// entirely.
    ///
    /// Anonymous authentication belongs to the configuration an application
    /// applies by default.
    pub fn anonymous<F>(&mut self, anonymous: F) -> &mut Self
    where
        F: FnMut(&mut AnonymousConfigurer<Self>),
    {
        self.get_or_apply(AnonymousConfigurer::default(), anonymous);

        self
    }

    /// Ok
    ///
    /// Adds form based authentication.
    ///
    /// The generated form posts the username and the password to the processing
    /// URL, and an authenticated user is sent back to the page that was requested
    /// before. The closure changes the parameter names and the URLs; everything it
    /// does not touch keeps the default the configurer starts with.
    pub fn form_login<F>(&mut self, form_login: F) -> &mut Self
    where
        F: FnMut(&mut FormLoginConfigurer<Self>),
    {
        self.get_or_apply(FormLoginConfigurer::default(), form_login);

        self
    }

    /// Pending
    ///
    /// Adds SAML 2.0 relying party authentication.
    ///
    /// The relying party registrations the chain authenticates with have to be
    /// shared with this builder, and the closure is where the registrations, the
    /// user details service and the assertion verifier are configured.
    ///
    /// The chain is complete — a login can be started, the posted response is
    /// parsed, verified, and turned into an authentication — but no assertion
    /// verifier ships with the port: the default one refuses every response, and
    /// an application has to supply one that checks the signature. Until it does,
    /// the endpoint starts logins that can never complete. See
    /// [`Saml2LoginConfigurer`] for what the configurer contributes.
    pub fn saml2_login<F>(&mut self, saml2_login: F) -> &mut Self
    where
        F: FnMut(&mut Saml2LoginConfigurer<Self>),
    {
        self.get_or_apply(Saml2LoginConfigurer::default(), saml2_login);

        self
    }

    /// Pending
    ///
    /// Adds SAML 2.0 single logout.
    ///
    /// The relying party registrations have to be shared with this builder, and
    /// the closure configures the relying party initiated logout and the handling
    /// of the logout responses an asserting party sends back.
    ///
    /// The endpoints are not wired yet, and the reason is the same one that
    /// keeps [`Self::saml2_login`] from completing a login: a logout request and
    /// its response carry a signature of the asserting party, and the port has no
    /// XML signature implementation to check it. The configurer holds its
    /// settings but contributes no filter to the chain.
    pub fn saml2_logout<F>(&mut self, saml2_logout: F) -> &mut Self
    where
        F: FnMut(&mut Saml2LogoutConfigurer<Self>),
    {
        let configurer = Saml2LogoutConfigurer::new(self.get_context());
        self.get_or_apply(configurer, saml2_logout);

        self
    }

    /// Ok
    ///
    /// Adds the SAML 2.0 metadata endpoint.
    ///
    /// The endpoint publishes the relying party registrations of the application
    /// as an entity descriptor, which is the document an asserting party is handed
    /// when the two sides are set up. A request that names a registration is
    /// answered with that registration, a request without one with every
    /// registration; an unknown registration id is answered with 401.
    pub fn saml2_metadata<F>(&mut self, saml2_metadata: F) -> &mut Self
    where
        F: FnMut(&mut Saml2MetadataConfigurer<Self>),
    {
        let configurer = Saml2MetadataConfigurer::new(self.get_context());
        self.get_or_apply(configurer, saml2_metadata);

        self
    }

    /// Ok
    ///
    /// Configures login through an OAuth 2.0 or OpenID Connect 1.0 provider.
    ///
    /// The chain performs the authorization code flow: a request that needs an
    /// authentication is redirected to the provider, the code that comes back is
    /// exchanged for tokens, and the user information those tokens carry becomes
    /// the authentication. The client registrations have to be shared with this
    /// builder.
    ///
    /// `/oauth2/authorization/{registrationId}` starts the login of a registration
    /// and remembers the authorization request it issued, the callback the
    /// provider sends the user back to answers it, and the login is stored as the
    /// user the profile names, with the authorities the configured mapper decides.
    /// A profile that cannot be loaded signs nobody in, and a callback that answers
    /// a request the application never issued is refused. The login page of the
    /// default configuration links every registration by name. OpenID Connect
    /// adds an `id_token` to the exchange: the validator and the provider of the
    /// module are tested on their own, the end-to-end tests of the configuration
    /// cover the OAuth 2.0 login.
    #[cfg(feature = "oauth2-client")]
    pub fn oauth2_login<F>(&mut self, oauth2_login: F) -> &mut Self
    where
        F: FnMut(&mut OAuth2LoginConfigurer<Self>),
    {
        self.get_or_apply(OAuth2LoginConfigurer::default(), oauth2_login);

        self
    }

    /// Pending
    ///
    /// Configures OpenID Connect logout.
    ///
    /// The endpoint ends the session at the provider as well as in the
    /// application, so a user who logged out is not silently authenticated again
    /// by the provider on the next request. The closure configures the endpoint
    /// and the redirect that follows it.
    #[cfg(all(feature = "oauth2-client", feature = "oauth2-resource-server"))]
    pub fn oidc_logout<F>(&mut self, oidc_logout: F) -> &mut Self
    where
        F: FnMut(&mut OidcLogoutConfigurer<Self>),
    {
        self.get_or_apply(OidcLogoutConfigurer::default(), oidc_logout);

        self
    }

    /// Ok
    ///
    /// Adds OAuth 2.0 client support.
    ///
    /// The chain drives the authorization code grant for the registered clients,
    /// and stores the authorization it obtained so a later request of the same
    /// user can use it. The closure configures the repository those authorizations
    /// are stored in.
    ///
    /// A request for `/oauth2/authorization/{registrationId}` is answered with a
    /// redirect to the authorization endpoint of that client, and the callback the
    /// provider sends the user back to is exchanged for a token: the application
    /// then holds the authorized client, with the refresh token the provider
    /// issued. The registrations and the authorized clients are shared objects, so
    /// the rest of the chain reads the same ones. See [`OAuth2ClientConfigurer`]
    /// for the filters the configuration contributes.
    #[cfg(feature = "oauth2-client")]
    pub fn oauth2_client<F>(&mut self, oauth2_client: F) -> &mut Self
    where
        F: FnMut(&mut OAuth2ClientConfigurer<Self>),
    {
        self.get_or_apply(OAuth2ClientConfigurer::default(), oauth2_client);

        self
    }

    /// Ok
    ///
    /// Adds OAuth 2.0 resource server support.
    ///
    /// The chain authenticates a request by the bearer token it carries and turns
    /// the claims of the token into the authentication, so a rule can require an
    /// authority or a scope the token was issued with. The closure decides how a
    /// token is decoded and validated.
    ///
    /// A request that carries a token the decoder accepts continues down the
    /// chain as the user the token names, with the scopes of the token as
    /// authorities; a request without a token is left to the application, and a
    /// token the decoder refuses is answered with 401. See
    /// [`OAuth2ResourceServerConfigurer`] for the entry point and the handler the
    /// configuration installs.
    #[cfg(feature = "oauth2-resource-server")]
    pub fn oauth2_resource_server<F>(&mut self, oauth2_resource_server: F) -> &mut Self
    where
        F: FnMut(&mut OAuth2ResourceServerConfigurer<Self>),
    {
        self.get_or_apply(
            OAuth2ResourceServerConfigurer::default(),
            oauth2_resource_server,
        );

        self
    }

    /// Pending
    ///
    /// Adds OAuth 2.1 authorization server support.
    ///
    /// The chain serves the authorization, token, revocation and introspection
    /// endpoints of an authorization server, and the configurer is where the
    /// registered clients, the stored authorizations and the token generator are
    /// configured.
    #[cfg(feature = "oauth2-authorization-server")]
    pub fn oauth2_authorization_server<F>(&mut self, oauth2_authorization_server: F) -> &mut Self
    where
        F: FnMut(&mut OAuth2AuthorizationServerConfigurer<Self>),
    {
        self.get_or_apply(
            OAuth2AuthorizationServerConfigurer::default(),
            oauth2_authorization_server,
        );

        self
    }

    /// Ok
    ///
    /// Adds one-time token login.
    ///
    /// A request to the generation endpoint issues a token and delivers it to the
    /// user out of band, and the token submitted to the login page authenticates
    /// the user it was issued for. The closure configures the endpoints and the
    /// service that issues and validates the tokens.
    ///
    /// The user details service shared with this builder is installed on the
    /// configurer when the closure did not install one, because the
    /// authentication of a token loads the user with it. A success handler is
    /// required, and a configuration without one fails at startup.
    pub fn one_time_token_login<F>(&mut self, one_time_token_login: F) -> &mut Self
    where
        F: FnMut(&mut OneTimeTokenLoginConfigurer<Self>),
    {
        let mut configurer = OneTimeTokenLoginConfigurer::new(self.get_context());
        // The user details service of the application is shared by the HTTP
        // security, because the one-time token authentication loads the user of
        // a token with it. The value of the DSL takes precedence over it.
        if configurer.get_user_details_service().is_none() {
            if let Some(user_details_service) = self
                .shared_object::<Arc<dyn crate::core::userdetails::UserDetailsService>>()
                .cloned()
            {
                configurer.user_details_service(user_details_service);
            }
        }
        self.get_or_apply(configurer, one_time_token_login);

        self
    }

    /// Ok
    ///
    /// Redirects requests to HTTPS.
    ///
    /// Every request the chain matches is redirected by default, and the closure
    /// narrows the requests that require the secure channel. The redirect uses the
    /// mapper configured by [`Self::port_mapper`], so a deployment that terminates
    /// TLS on another port has to configure the mapper as well.
    pub fn redirect_to_https<F>(&mut self, redirect_to_https: F) -> &mut Self
    where
        F: FnMut(&mut HttpsRedirectConfigurer<Self>),
    {
        self.get_or_apply(HttpsRedirectConfigurer::default(), redirect_to_https);

        self
    }

    /// Ok
    ///
    /// Adds HTTP Basic authentication.
    ///
    /// The chain answers a request without an authentication with a challenge, and
    /// a client that sends the credentials in the authorization header is
    /// authenticated with them. The closure changes the realm — `Realm` by default
    /// — and the entry point that sends the challenge.
    pub fn http_basic<F>(&mut self, http_basic: F) -> &mut Self
    where
        F: FnMut(&mut HttpBasicConfigurer<Self>),
    {
        self.get_or_apply(HttpBasicConfigurer::default(), http_basic);

        self
    }

    /// Ok
    ///
    /// Adds password management.
    ///
    /// The chain serves the page a user changes the password on, and applies the
    /// change through the password encoder and the user details service of the
    /// application. The closure changes the page that is served and the handling
    /// around it.
    pub fn password_management<F>(&mut self, password_management: F) -> &mut Self
    where
        F: FnMut(&mut PasswordManagementConfigurer<Self>),
    {
        self.get_or_apply(PasswordManagementConfigurer::default(), password_management);

        self
    }

    /// Ok
    ///
    /// Adds X.509 client certificate authentication.
    ///
    /// The principal of the authentication is taken from the certificate the
    /// client presented, so the chain has to be reached over a connection that
    /// requested a client certificate. The closure configures which part of the
    /// subject is used as the username.
    ///
    /// The certificate itself is validated where the connection is made: the
    /// container, or the terminator in front of it, checks the chain and writes
    /// what it accepted into the request attribute this port reads
    /// (`next.http_request.X509_certificate`). A refused certificate therefore
    /// never reaches the chain, and this method never trusts a certificate on
    /// its own.
    pub fn x509<F>(&mut self, x509: F) -> &mut Self
    where
        F: FnMut(&mut X509Configurer<Self>),
    {
        self.get_or_apply(X509Configurer::default(), x509);

        self
    }

    /// Sets the authentication manager the chain authenticates with.
    ///
    /// Without a manager of its own, the chain builds one from the authentication
    /// builder this instance was created with, which is what applies the providers
    /// the application configured. A manager set here replaces that build.
    pub fn authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) -> &mut Self {
        self.authentication_manager = Some(authentication_manager);

        self
    }

    /// Ok
    ///
    /// Adds matchers for the requests this builder applies to.
    ///
    /// The matchers the closure declares are added to the ones already declared,
    /// and the builder applies to a request that matches any of them. A closure
    /// that declares several matchers therefore widens the set, whereas
    /// [`Self::security_matcher`] replaces it.
    ///
    /// The matcher the calls leave behind is the one the chain matches requests
    /// with, so a chain built this way serves the requests its matchers declare
    /// and lets the chains after it serve the rest. A closure that declares no
    /// matcher leaves the matcher of the builder as it was, neither widening nor
    /// narrowing it.
    pub fn security_matchers<F>(&mut self, mut security_matchers: F) -> &mut Self
    where
        F: FnMut(&mut RequestMatcherConfigurer),
    {
        security_matchers(&mut self.request_matcher_configurer);
        let security_matcher = self.request_matcher_configurer.or_request_matcher();
        self.security_matcher(security_matcher)
    }

    /// Ok
    ///
    /// Adds WebAuthn (passkeys) registration and authentication.
    ///
    /// The closure configures the relying party the application presents itself
    /// as: its identifier, its name, and the origins a ceremony may run on. A
    /// relying party identifier is required — without one a credential has
    /// nothing to be bound to — and so is a `UserDetailsService`, which loads the
    /// user a credential belongs to.
    ///
    /// The chain then serves the two ceremonies of the reference implementation:
    /// `POST /webauthn/register/options` and `POST /webauthn/register` register a
    /// passkey for the user the request is authenticated as, and
    /// `POST /webauthn/authenticate/options` and `POST /webauthn/authenticate`
    /// log a user in with one. The generated login page offers the passkey
    /// sign-in, `GET /webauthn/register` serves the page a passkey is registered
    /// from, and `/login/webauthn.js` carries the script both pages call — an
    /// application with its own pages includes that script and calls
    /// `setupLogin`/`setupRegistration` itself. The ceremony itself — the
    /// challenge, the origin, the relying party, the COSE public key and the
    /// signature over `authenticatorData || SHA-256(clientDataJSON)` — is
    /// [`crate::webauthn::WebAuthnManager`], and only the `none` attestation
    /// format is believed.
    pub fn web_authn<F>(&mut self, web_authn: F) -> &mut Self
    where
        F: FnMut(&mut WebAuthnConfigurer<Self>),
    {
        self.get_or_apply(WebAuthnConfigurer::default(), web_authn);

        self
    }

    /// Applies this builder only to the requests that match the given matcher.
    ///
    /// The matcher replaces every matcher declared on this builder before, so it
    /// is the one place the scope of the chain is set when a single matcher is
    /// enough. A builder that was never given a matcher applies to every request.
    pub fn security_matcher(&mut self, request_matcher: Arc<dyn RequestMatcher>) -> &mut Self {
        self.request_matcher = request_matcher;
        self
    }

    /// Applies this builder only to the requests that match one of the given
    /// patterns.
    ///
    /// A request that matches any of the patterns selects this chain. The matcher
    /// is built only when a [`PathPatternRequestMatcherBuilder`] was shared with
    /// this builder, so a builder created without one keeps the matcher it had
    /// instead of failing.
    pub fn security_matcher_with_strs(&mut self, patterns: &[&str]) -> &mut Self {
        if let Some(builder) = self.get_shared_object::<PathPatternRequestMatcherBuilder>() {
            let matchers = patterns
                .into_iter()
                .map(|s| builder.matcher(None, s))
                .map(|v| Arc::new(v) as Arc<dyn RequestMatcher>)
                .collect::<Vec<_>>();

            self.request_matcher = Arc::new(OrRequestMatcher::new(matchers));
        }

        self
    }

    /// Adds `filter` at the position of the filter `F1`.
    ///
    /// The position is the one the filter registration gives `F1`, so the filter
    /// runs where `F1` runs. Two filters added at the same position keep no
    /// defined order relative to each other, which is why a filter that has to run
    /// before or after another one is added with the corresponding before/after
    /// method instead.
    pub fn add_filter_at<F, F1>(&mut self, filter: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(filter, 0);
    }

    /// Ok
    ///
    /// Returns the authentication builder the chain builds its authentication
    /// manager from.
    ///
    /// The builder is the one this instance was created with, and it is the place a
    /// provider or a user details service is registered on: a provider registered
    /// through it is the one the manager the build shares authenticates with.
    /// [`Self::authentication_manager`] replaces that build, and then the manager
    /// it was given is the one the chain authenticates with.
    pub fn get_authentication_registry(&mut self) -> Option<&mut AuthenticationManagerBuilder> {
        self.shared_object_mut::<AuthenticationManagerBuilder>()
    }
}

impl SecurityBuilder<DefaultSecurityFilterChain> for HttpSecurity {
    /// Builds the filter chain the configurers describe.
    ///
    /// The configurers are initialized and then configured in one pass, and the
    /// filters they contributed are ordered into the chain.
    ///
    /// # Panics
    ///
    /// Panics when the builder was already built. A chain is the product of a
    /// single build, and building the same builder twice would run the configurers
    /// a second time over the state the first build left behind.
    fn build(&mut self) -> DefaultSecurityFilterChain {
        assert!(
            !self
                .base
                .base_security_builder
                .building
                .swap(true, Ordering::SeqCst),
            "This object has already been built"
        );

        self.base.build_state = BuildState::INITIALIZING;
        self.before_init();
        BaseConfiguredSecurityBuilder::init_configurers(self);
        self.base.build_state = BuildState::CONFIGURING;
        self.before_configure();
        BaseConfiguredSecurityBuilder::configure_configurers(self);
        self.base.build_state = BuildState::BUILDING;
        let result = self.perform_build();
        self.base.build_state = BuildState::BUILT;

        result
    }
}

impl BaseConfiguredSecurityBuilderExt<DefaultSecurityFilterChain, Self> for HttpSecurity {
    fn before_init(&mut self) {}

    /// Shares the authentication manager the chain authenticates with.
    ///
    /// The manager is the one set on this builder, or the one the authentication
    /// builder produces, so a configurer that authenticates a request finds a
    /// manager among the shared objects either way.
    fn before_configure(&mut self) {
        let manager = match self.authentication_manager.as_ref() {
            Some(manager) => Some(manager.clone()),
            None => self.get_authentication_registry().map(|s| s.build()),
        };

        manager.map(|m| self.set_shared_object(m));
    }

    /// Orders the filters and produces the chain.
    ///
    /// The filters are sorted by the order they were registered with, and the chain
    /// keeps the matcher of this builder, so the chain only sees the requests this
    /// builder applies to.
    fn perform_build(&mut self) -> DefaultSecurityFilterChain {
        self.filters.sort_by_key(|c| c.order);
        let sorted_filters = self
            .filters
            .iter()
            .map(|c| c.filter.clone())
            .collect::<Vec<_>>();

        DefaultSecurityFilterChain::new(self.request_matcher.clone(), sorted_filters)
    }
}

impl HttpSecurityBuilder<Self> for HttpSecurity {
    fn configurer<C>(&self) -> Option<&C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        C: 'static,
    {
        self.base.configurer()
    }

    fn configurer_mut<C>(&mut self) -> Option<&mut C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        C: 'static,
    {
        self.base.configurer_mut()
    }

    fn remove_configurer<C>(&mut self) -> Option<C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, Self>,
        C: 'static,
    {
        self.base.remove_configurer()
    }

    fn authentication_provider(
        &mut self,
        authentication_provider: Arc<dyn AuthenticationProvider>,
    ) {
        self.get_authentication_registry()
            .map(|builder| builder.authentication_provider(authentication_provider));
    }

    fn user_details_service<T>(&mut self, user_details_service: T)
    where
        T: UserDetailsService + 'static,
    {
        self.get_authentication_registry()
            .map(|r| r.user_details_service(Arc::new(user_details_service)));
    }

    fn set_shared_object<C>(&mut self, object: C)
    where
        C: AnyClone,
    {
        self.base.set_shared_object(object);
    }

    fn shared_object<T>(&self) -> Option<&T>
    where
        T: AnyClone,
    {
        self.base.get_shared_object()
    }

    fn shared_object_mut<T>(&mut self) -> Option<&mut T>
    where
        T: AnyClone,
    {
        self.base.get_mut_shared_object()
    }

    fn add_filter<F: HttpFilter>(&mut self, filter: F) {
        let name = std::any::type_name::<F>();
        let order = self.filter_orders.get_order_by_name(name).unwrap_or_else(|| {
            panic!("Filter {name} has no registered order. Use add_filter_before or add_filter_after.")
        });
        self.filters
            .push(OrderedFilter::new(Arc::new(filter), order));
    }

    fn add_filter_after<F, F1>(&mut self, f: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(f, 1);
    }

    fn add_filter_before<F, F1>(&mut self, f: F)
    where
        F: HttpFilter,
        F1: HttpFilter,
    {
        self.add_filter_at_offset::<F, F1>(f, -1);
    }
}

impl Clone for HttpSecurity {
    fn clone(&self) -> Self {
        Self {
            request_matcher_configurer: self.request_matcher_configurer.clone(),
            filters: self.filters.clone(),
            request_matcher: self.request_matcher.clone(),
            filter_orders: self.filter_orders.clone(),
            authentication_manager: self.authentication_manager.clone(),

            base: self.base.clone(),
        }
    }
}

impl Deref for HttpSecurity {
    type Target = BaseConfiguredSecurityBuilder<DefaultSecurityFilterChain, Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for HttpSecurity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Collects the matchers of the requests a [`HttpSecurity`] applies to.
///
/// The registry is what the closure of `security_matchers` receives, and it is
/// what turns the matchers declared there into the single matcher the chain is
/// built with.
#[derive(Clone)]
pub struct RequestMatcherConfigurer {
    base: BaseRequestMatcherRegistry<Self>,
}

impl RequestMatcherConfigurer {
    /// Creates a registry whose matchers are created with the given builder.
    pub fn new(builder: PathPatternRequestMatcherBuilder) -> Self {
        let mut base = BaseRequestMatcherRegistry::<Self>::default();
        base.set_request_matcher_builder(builder);

        Self { base }
    }

    /// Replaces the matchers the registry holds.
    pub fn set_matchers(&mut self, matchers: Vec<Arc<dyn RequestMatcher>>) {
        self.base._req_matchers = matchers;
    }

    /// Returns the single matcher of the requests that match any held matcher.
    fn or_request_matcher(&self) -> Arc<dyn RequestMatcher> {
        Arc::new(OrRequestMatcher::new(self.base._req_matchers.clone()))
    }
}

impl Deref for RequestMatcherConfigurer {
    type Target = BaseRequestMatcherRegistry<Self>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for RequestMatcherConfigurer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// A filter together with the order it is placed at in the chain.
///
/// The chain sorts the filters it holds by this order before it is built, so the
/// order is what decides where a filter runs. The wrapper delegates the filtering
/// to the filter it wraps, which is the filter the chain actually invokes.
#[derive(Clone)]
pub struct OrderedFilter {
    filter: Arc<dyn HttpFilter>,
    order: i32,
}

impl OrderedFilter {
    /// Wraps a filter with the order it is placed at.
    fn new(filter: Arc<dyn HttpFilter>, order: i32) -> Self {
        Self { filter, order }
    }
}

impl Ordered for OrderedFilter {
    /// Returns the order the filter is sorted with.
    fn order(&self) -> i32 {
        self.order
    }
}

#[async_trait]
impl HttpFilter for OrderedFilter {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        self.filter.do_filter(req, resp, chain).await
    }
}

impl Named for OrderedFilter {
    fn name(&self) -> &str {
        "OrderedFilter"
    }
}

impl Display for OrderedFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(filter={}, order={})",
            self.name(),
            self.filter.name(),
            self.order
        )
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use next_web_core::http::HttpMethod;

    use super::*;
    use crate::web::util::matcher::PathPatternRequestMatcherBuilder;

    /// Returns a `GET` request for the given path.
    fn request(path: &str) -> Request {
        Request::builder()
            .method(HttpMethod::GET)
            .uri(path)
            .body(Body::empty())
            .expect("a valid request")
    }

    /// Returns an HTTP security that shares the given request matcher builder.
    fn http_with_matcher_builder(builder: PathPatternRequestMatcherBuilder) -> HttpSecurity {
        let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(builder),
        );

        HttpSecurity::new(AuthenticationManagerBuilder::default(), shared_objects)
    }

    #[test]
    fn the_shared_request_matcher_builder_is_used() {
        let builder = PathPatternRequestMatcherBuilder::default().base_path("/spring");
        let mut http = http_with_matcher_builder(builder);

        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/admin/**"]);
        });

        assert!(
            http.request_matcher
                .matches(&request("/spring/admin/users")),
            "the base path of the shared builder is prepended"
        );
        assert!(
            !http.request_matcher.matches(&request("/admin/users")),
            "the unprefixed path is not the one the chain applies to"
        );
    }

    #[test]
    fn the_matchers_of_several_calls_are_added() {
        let builder = PathPatternRequestMatcherBuilder::default();
        let mut http = http_with_matcher_builder(builder);

        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/admin/**"]);
        });
        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/api/**"]);
        });

        assert!(
            http.request_matcher.matches(&request("/admin/users")),
            "the matcher of the first call still applies"
        );
        assert!(
            http.request_matcher.matches(&request("/api/users")),
            "the matcher of the second call applies as well"
        );
        assert!(
            !http.request_matcher.matches(&request("/login")),
            "a request that matches neither matcher is out of scope"
        );
    }

    #[test]
    fn an_empty_closure_keeps_the_matcher_of_the_builder() {
        let builder = PathPatternRequestMatcherBuilder::default();
        let mut http = http_with_matcher_builder(builder);
        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/admin/**"]);
        });

        http.security_matchers(|_configurer| {});

        assert!(
            http.request_matcher.matches(&request("/admin/users")),
            "a closure that declares no matcher does not widen the scope of the chain"
        );
        assert!(
            !http.request_matcher.matches(&request("/login")),
            "a closure that declares no matcher does not narrow the scope either"
        );
    }

    #[test]
    fn the_built_chain_only_serves_the_requests_the_matchers_declare() {
        use crate::{config::security_builder::SecurityBuilder, web::security_filter_chain::SecurityFilterChain};

        let builder = PathPatternRequestMatcherBuilder::default();
        let mut http = http_with_matcher_builder(builder);
        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/admin/**"]);
        });

        let chain = http.build();

        assert!(
            chain.matches(&mut request("/admin/users")),
            "the chain serves a request its matchers declare"
        );
        assert!(
            !chain.matches(&mut request("/login")),
            "the chain leaves every other request to the chains after it"
        );
    }

    #[test]
    fn a_single_matcher_replaces_the_matchers_declared_before_it() {
        let builder = PathPatternRequestMatcherBuilder::default();
        let mut http = http_with_matcher_builder(builder);
        http.security_matchers(|configurer| {
            configurer.request_matchers(vec!["/admin/**"]);
        });

        http.security_matcher(Arc::new(PathPatternRequestMatcherBuilder::default().matcher(
            None,
            "/api/**",
        )));

        assert!(
            http.request_matcher.matches(&request("/api/users")),
            "the replacing matcher applies"
        );
        assert!(
            !http.request_matcher.matches(&request("/admin/users")),
            "the matchers declared before it are gone"
        );
    }

    /// A provider that accepts every token it supports, so that the manager of a
    /// chain can be told apart from one that was built another way.
    #[derive(Clone)]
    struct AcceptingProvider;

    #[next_web_core::async_trait]
    impl crate::authentication::AuthenticationProvider for AcceptingProvider {
        async fn authenticate(
            &self,
            authentication: &Arc<dyn crate::core::Authentication>,
        ) -> Result<Option<Arc<dyn crate::core::Authentication>>, crate::core::AuthenticationError>
        {
            Ok(Some(authentication.clone()))
        }

        fn supports(&self, _authentication: std::any::TypeId) -> bool {
            true
        }
    }

    /// A manager that authenticates nobody, so that a chain configured with it
    /// can be told apart from the one its registry builds.
    struct DenyingManager;

    #[next_web_core::async_trait]
    impl crate::authorization::AuthenticationManager for DenyingManager {
        async fn authenticate(
            &self,
            _authentication: &dyn crate::core::Authentication,
        ) -> Result<Arc<dyn crate::core::Authentication>, crate::core::AuthenticationError> {
            Err(crate::core::AuthenticationError::new(
                "no provider of this manager accepts the token",
            ))
        }
    }

    /// Returns a token of the user the tests name.
    fn token() -> Arc<dyn crate::core::Authentication> {
        Arc::new(
            crate::web::authentication::preauth::PreAuthenticatedAuthenticationToken::new(
                Arc::new("Alice".to_string()),
                None,
            ),
        )
    }

    #[tokio::test]
    async fn the_registry_builds_the_manager_the_chain_authenticates_with() {
        use crate::authorization::AuthenticationManager;
        use crate::config::authentication::provider_manager_builder::ProviderManagerBuilder;
        use crate::config::security_builder::SecurityBuilder;

        let mut http = http_with_matcher_builder(PathPatternRequestMatcherBuilder::default());
        http.get_authentication_registry()
            .expect("a builder this instance created holds a registry")
            .authentication_provider(Arc::new(AcceptingProvider));
        let _chain = http.build();
        let manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .expect("the build shares the manager it built");

        let authenticated = manager
            .authenticate(&*token())
            .await
            .expect("the provider of the registry accepts the token");

        assert_eq!(
            authenticated.name(),
            "Alice",
            "the manager authenticates with the provider that was registered on the registry"
        );
    }

    #[tokio::test]
    async fn a_manager_of_its_own_replaces_the_one_the_registry_builds() {
        use crate::authorization::AuthenticationManager;
        use crate::config::authentication::provider_manager_builder::ProviderManagerBuilder;
        use crate::config::security_builder::SecurityBuilder;

        let mut http = http_with_matcher_builder(PathPatternRequestMatcherBuilder::default());
        http.get_authentication_registry()
            .expect("a builder this instance created holds a registry")
            .authentication_provider(Arc::new(AcceptingProvider));
        http.authentication_manager(Arc::new(DenyingManager));
        let _chain = http.build();
        let manager = http
            .shared_object::<Arc<dyn AuthenticationManager>>()
            .expect("the build shares the manager it was given");

        let outcome = manager.authenticate(&*token()).await;

        assert!(
            outcome.is_err(),
            "the manager of the builder is the one the chain authenticates with, not the one of the registry"
        );
    }
}
