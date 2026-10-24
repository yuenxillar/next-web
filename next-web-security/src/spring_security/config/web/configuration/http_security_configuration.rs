//! Provides the HTTP security the security filter chains are built from.

use std::{any::TypeId, collections::HashMap, error::Error, sync::Arc};

use next_web_context::{ApplicationContextExt, ApplicationEventPublisher};
use next_web_core::{
    any::AnyClone,
    async_trait,
    config::AutoConfiguration,
    error::BoxError,
    web::{
        accept::{ContentNegotiationStrategy, HeaderContentNegotiationStrategy},
        cors::UrlBasedCorsConfigurationSource,
    },
    ApplicationContext, Ordered,
};
use next_web_macros::singleton;

use crate::{
    access::{hierarchicalroles::RoleHierarchy, intercept::RequestAuthorizationContext},
    authentication::{
        dao::DaoAuthenticationProvider, AuthenticationEventPublisher,
        DefaultAuthenticationEventPublisher,
    },
    authorization::{AuthorizationEventPublisher, AuthorizationManagerFactory},
    config::{
        authentication::{
            builders::authentication_manager_builder::AuthenticationManagerBuilder,
            provider_manager_builder::ProviderManagerBuilder,
        },
        http::core::granted_authority_defaults::GrantedAuthorityDefaults,
        web::{
            builders::HttpSecurity,
            configurers::{DefaultLoginPageConfigurer, PreAuthorizeConfigurer},
            http_security_builder::HttpSecurityBuilder,
            WebSecurityConfigurer,
        },
    },
    core::userdetails::UserDetailsService,
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
    web::util::matcher::PathPatternRequestMatcherBuilder,
};

/// The name the HTTP security of the application is registered under.
///
/// The name is the one the framework derives from the type of [`HttpSecurity`],
/// so a filter chain can depend on the HTTP security without naming it.
pub const HTTP_SECURITY_SINGLETON_NAME: &str = "httpSecurity";

/// The name a password encoder is registered under by convention.
const PASSWORD_ENCODER_SINGLETON_NAME: &str = "passwordEncoder";

/// Provides the [`HttpSecurity`] the filter chains of the application are built
/// from.
///
/// The HTTP security is created with the configuration the framework uses by
/// default: CSRF protection, exception handling, the security headers, session
/// management, the security context, the request cache, anonymous
/// authentication, the default login page, and logout are applied. CORS is
/// applied as well when the application contributes a CORS configuration
/// source.
///
/// The values a filter chain is built with, such as the request matcher
/// builder, the authorization manager factory, the role hierarchy, and the
/// publishers, are resolved from the application once and shared with the HTTP
/// security. Sharing them is what lets the filter chains be built without
/// holding the application context.
///
/// An application can take over the HTTP security by registering an
/// [`HttpSecurity`] under [`HTTP_SECURITY_SINGLETON_NAME`] itself; this
/// configuration then leaves it alone.
///
/// # Examples
///
/// ```ignore
/// // The filter chain of the application is declared with a provider that
/// // depends on the HTTP security the configuration contributes.
/// #[configuration]
/// impl SecurityConfiguration {
///     #[provider]
///     fn security_filter_chain(http_security: HttpSecurity) -> Arc<dyn SecurityFilterChain> {
///         let mut http_security = http_security;
///         http_security.authorize_http_requests(|requests| {
///             requests.any_request().permit_all();
///         });
///
///         Arc::new(http_security.build())
///     }
/// }
/// ```
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct HttpSecurityConfiguration;

impl HttpSecurityConfiguration {
    /// Hands the configuration to the context as an auto-configuration.
    #[must_use]
    pub fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }

    /// Creates the HTTP security of the application.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the values the HTTP security is configured with are
    ///   resolved from.
    fn http_security(ctx: &mut dyn ApplicationContext) -> HttpSecurity {
        let mut authentication_builder = AuthenticationManagerBuilder::default();
        authentication_builder.authentication_event_publisher(authentication_event_publisher(ctx));

        let mut http = HttpSecurity::new(authentication_builder, create_shared_objects(ctx));

        // The configuration the application applies by default.
        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .session_management(|_| {})
            .security_context(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {});
        http.with(DefaultLoginPageConfigurer::default(), |_| {});
        // The rules a handler is guarded with read the authorities of the
        // request, so the authorities of the authentication the chain
        // established are written into the request by this configurer.
        http.with(PreAuthorizeConfigurer::default(), |_| {});
        http.logout(|_| {});

        apply_cors_if_available(ctx, &mut http);

        // The configurers of the application run last, so that the rules they
        // declare take precedence over the default ones.
        for mut configurer in ctx.resolve_by_type::<Box<dyn WebSecurityConfigurer>>() {
            configurer.configure(&mut http);
        }

        // The users of the application are authenticated with a provider built
        // from the user details service of the application, unless the
        // application configured an authentication provider or a parent
        // authentication manager of its own.
        apply_default_authentication_provider(ctx, &mut http);

        http
    }
}

/// Registers the authentication provider that authenticates a user of the
/// application.
///
/// The provider is built from the user details service of the application and
/// the password encoder the HTTP security shares, and it is registered only
/// when the application did not configure an authentication provider of its
/// own.
///
/// # Arguments
///
/// * `ctx` - The context the user details service is resolved from.
/// * `http` - The HTTP security the provider is registered with.
fn apply_default_authentication_provider(
    ctx: &mut dyn ApplicationContext,
    http: &mut HttpSecurity,
) {
    let Some(user_details_service) = ctx
        .resolve_by_type::<Arc<dyn UserDetailsService>>()
        .into_iter()
        .next()
    else {
        return;
    };

    let password_encoder = http.shared_object::<Arc<dyn PasswordEncoder>>().cloned();
    let Some(builder) = http.shared_object_mut::<AuthenticationManagerBuilder>() else {
        return;
    };
    if builder.is_configured() {
        return;
    }

    let mut provider = DaoAuthenticationProvider::new(user_details_service);
    if let Some(password_encoder) = password_encoder {
        provider.set_password_encoder(password_encoder);
    }
    builder.authentication_provider(Arc::new(provider));
}

#[async_trait]
impl AutoConfiguration for HttpSecurityConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        if ctx.contains_singleton_with_default_name::<HttpSecurity>() {
            tracing::debug!(
                "the application registers its own HTTP security, so the configured one is not \
                 installed"
            );
            return Ok(());
        }

        let http_security = Self::http_security(ctx);
        ctx.insert_singleton_with_name(http_security, HTTP_SECURITY_SINGLETON_NAME);

        Ok(())
    }

    /// The HTTP security is contributed before the filter chains that depend on
    /// it are created.
    fn auto_configure_before(&self) -> &'static [&'static str] {
        &["WebSecurityAutoConfiguration"]
    }
}

impl Ordered for HttpSecurityConfiguration {
    fn order(&self) -> i32 {
        i32::MIN + 20
    }
}

/// Applies CORS when the application contributes a CORS configuration source.
///
/// # Arguments
///
/// * `ctx` - The context the CORS configuration source is looked up in.
/// * `http` - The HTTP security to configure.
fn apply_cors_if_available(ctx: &dyn ApplicationContext, http: &mut HttpSecurity) {
    if !ctx
        .get_singletons_by_type::<UrlBasedCorsConfigurationSource>()
        .is_empty()
    {
        http.cors(|_| {});
    }
}

/// Returns the publisher the authentication events are published to.
///
/// The publisher of the application is used when it contributes one, and a
/// publisher that publishes the events of the application event publisher of
/// the context is used otherwise.
///
/// # Arguments
///
/// * `ctx` - The context the publishers are resolved from.
fn authentication_event_publisher(
    ctx: &mut dyn ApplicationContext,
) -> Arc<dyn AuthenticationEventPublisher> {
    if let Some(publisher) = ctx.resolve_option::<Arc<dyn AuthenticationEventPublisher>>() {
        return publisher;
    }

    if let Some(publisher) = ctx.resolve_option::<Arc<dyn ApplicationEventPublisher>>() {
        return Arc::new(DefaultAuthenticationEventPublisher::new(publisher));
    }

    Arc::new(DefaultAuthenticationEventPublisher::default())
}

/// Resolves the values the HTTP security shares with the configurations of its
/// filter chains.
///
/// # Arguments
///
/// * `ctx` - The context the values are resolved from.
fn create_shared_objects(ctx: &mut dyn ApplicationContext) -> HashMap<TypeId, Box<dyn AnyClone>> {
    let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();

    shared_objects.insert(
        TypeId::of::<PathPatternRequestMatcherBuilder>(),
        Box::new(
            ctx.resolve_option::<PathPatternRequestMatcherBuilder>()
                .unwrap_or_default(),
        ),
    );

    shared_objects.insert(
        TypeId::of::<Arc<dyn ContentNegotiationStrategy>>(),
        Box::new(
            ctx.resolve_option::<Arc<dyn ContentNegotiationStrategy>>()
                .unwrap_or_else(|| Arc::new(HeaderContentNegotiationStrategy)),
        ),
    );

    shared_objects.insert(
        TypeId::of::<Arc<dyn PasswordEncoder>>(),
        Box::new(Arc::new(LazyPasswordEncoder::resolve(ctx)) as Arc<dyn PasswordEncoder>),
    );

    if let Some(publisher) = ctx.resolve_option::<Arc<dyn ApplicationEventPublisher>>() {
        shared_objects.insert(
            TypeId::of::<Arc<dyn ApplicationEventPublisher>>(),
            Box::new(publisher),
        );
    }

    if let Some(publisher) = ctx.resolve_option::<Arc<dyn AuthorizationEventPublisher>>() {
        shared_objects.insert(
            TypeId::of::<Arc<dyn AuthorizationEventPublisher>>(),
            Box::new(publisher),
        );
    }

    if let Some(factory) =
        ctx.resolve_option::<Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>>()
    {
        shared_objects.insert(
            TypeId::of::<Arc<dyn AuthorizationManagerFactory<RequestAuthorizationContext>>>(),
            Box::new(factory),
        );
    }

    // The user details service of the application is shared, because a
    // configurer that authenticates a user (such as the one-time token login)
    // loads the user with it.
    if let Some(user_details_service) =
        ctx.resolve_option::<Arc<dyn crate::core::userdetails::UserDetailsService>>()
    {
        shared_objects.insert(
            TypeId::of::<Arc<dyn crate::core::userdetails::UserDetailsService>>(),
            Box::new(user_details_service),
        );
    }

    if let Some(role_hierarchy) = ctx.resolve_option::<Arc<dyn RoleHierarchy>>() {
        shared_objects.insert(
            TypeId::of::<Arc<dyn RoleHierarchy>>(),
            Box::new(role_hierarchy),
        );
    }

    if let Some(defaults) = ctx.resolve_option::<GrantedAuthorityDefaults>() {
        shared_objects.insert(TypeId::of::<GrantedAuthorityDefaults>(), Box::new(defaults));
    }

    // The values of the authorization server are shared with the HTTP security
    // in the same way, so the endpoints can be served with the values the
    // application contributed to its context.
    #[cfg(feature = "oauth2-authorization-server")]
    {
        use crate::oauth2::server::authorization::{
            AuthorizationServerSettings, JwtSigner, OAuth2AuthorizationConsentService,
            OAuth2AuthorizationService, OAuth2TokenGenerator, RegisteredClientRepository,
        };

        if let Some(settings) = ctx.resolve_option::<AuthorizationServerSettings>() {
            shared_objects.insert(
                TypeId::of::<AuthorizationServerSettings>(),
                Box::new(settings),
            );
        }
        if let Some(registered_clients) =
            ctx.resolve_option::<Arc<dyn RegisteredClientRepository>>()
        {
            shared_objects.insert(
                TypeId::of::<Arc<dyn RegisteredClientRepository>>(),
                Box::new(registered_clients),
            );
        }
        if let Some(authorizations) = ctx.resolve_option::<Arc<dyn OAuth2AuthorizationService>>() {
            shared_objects.insert(
                TypeId::of::<Arc<dyn OAuth2AuthorizationService>>(),
                Box::new(authorizations),
            );
        }
        if let Some(authorization_consents) =
            ctx.resolve_option::<Arc<dyn OAuth2AuthorizationConsentService>>()
        {
            shared_objects.insert(
                TypeId::of::<Arc<dyn OAuth2AuthorizationConsentService>>(),
                Box::new(authorization_consents),
            );
        }
        if let Some(token_generator) = ctx.resolve_option::<Arc<dyn OAuth2TokenGenerator>>() {
            shared_objects.insert(
                TypeId::of::<Arc<dyn OAuth2TokenGenerator>>(),
                Box::new(token_generator),
            );
        }
        if let Some(jwt_signer) = ctx.resolve_option::<Arc<dyn JwtSigner>>() {
            shared_objects.insert(TypeId::of::<Arc<dyn JwtSigner>>(), Box::new(jwt_signer));
        }
    }

    shared_objects
}

/// The password encoder of the application.
///
/// The encoder the application registers under the `passwordEncoder` name is
/// used when it contributes one, and the default encoder is used otherwise.
/// The encoder is resolved once, when the HTTP security is created, so a
/// configuration that does not use a password does not resolve it.
#[derive(Clone)]
struct LazyPasswordEncoder {
    /// The encoder the application contributed, or the default encoder.
    password_encoder: Arc<dyn PasswordEncoder>,
}

impl LazyPasswordEncoder {
    /// Resolves the password encoder of the application.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context the password encoder is resolved from.
    fn resolve(ctx: &mut dyn ApplicationContext) -> Self {
        let password_encoder = ctx
            .resolve_option_with_name::<Arc<dyn PasswordEncoder>>(PASSWORD_ENCODER_SINGLETON_NAME)
            .unwrap_or_else(|| Arc::new(BCryptPasswordEncoder::default()));

        Self { password_encoder }
    }
}

impl PasswordEncoder for LazyPasswordEncoder {
    fn encode(&self, raw_password: Option<&str>) -> Result<Option<String>, BoxError> {
        self.password_encoder.encode(raw_password)
    }

    fn upgrade_encoding(&self, encoded_password: &str) -> Result<bool, BoxError> {
        self.password_encoder.upgrade_encoding(encoded_password)
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        self.password_encoder
            .matches(raw_password, encoded_password)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use next_web_context::ApplicationContextExt;

    use super::{apply_default_authentication_provider, HTTP_SECURITY_SINGLETON_NAME};
    use crate::{
        config::{
            authentication::builders::authentication_manager_builder::AuthenticationManagerBuilder,
            web::{
                builders::{HttpSecurity, StandaloneApplicationContext},
                http_security_builder::HttpSecurityBuilder,
            },
        },
        core::userdetails::{User, UserDetailsService},
        provisioning::InMemoryUserDetailsManager,
    };

    /// Returns the user details service of a test.
    fn user_details_service() -> Arc<dyn UserDetailsService> {
        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username("user")
                .password(Some("password".to_string()))
                .build(),
        )]))
    }

    #[test]
    fn names_the_http_security_after_the_type() {
        // The name is the one the framework derives from the type, which is
        // what `contains_singleton_with_default_name` looks up.
        assert_eq!(HTTP_SECURITY_SINGLETON_NAME, "httpSecurity");
    }

    #[test]
    fn the_users_of_the_application_are_authenticated_with_its_user_details_service() {
        let mut context = StandaloneApplicationContext::default();
        context.insert_singleton_with_name(user_details_service(), "userDetailsService");
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());

        apply_default_authentication_provider(&mut context, &mut http);

        let builder = http
            .shared_object::<AuthenticationManagerBuilder>()
            .expect("the builder is shared");
        assert!(
            builder.is_configured(),
            "the provider of the user details service is registered"
        );
    }

    #[test]
    fn an_application_without_a_user_details_service_is_left_alone() {
        let mut context = StandaloneApplicationContext::default();
        let mut http = HttpSecurity::new(AuthenticationManagerBuilder::default(), HashMap::new());

        apply_default_authentication_provider(&mut context, &mut http);

        let builder = http
            .shared_object::<AuthenticationManagerBuilder>()
            .expect("the builder is shared");
        assert!(!builder.is_configured());
    }
}
