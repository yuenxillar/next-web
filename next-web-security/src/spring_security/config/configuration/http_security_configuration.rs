use std::{
    any::TypeId,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_context::ApplicationContextExt;
use next_web_core::{
    traits::any_clone::AnyClone,
    web::{accept::ContentNegotiationStrategy, cors::UrlBasedCorsConfigurationSource},
    ApplicationContext,
};

use crate::{
    authentication::{AuthenticationEventPublisher, DefaultAuthenticationEventPublisher},
    authorization::AuthenticationManager,
    config::{
        authentication::{
            builders::authentication_manager_builder::AuthenticationManagerBuilder,
            configuration::AuthenticationConfiguration,
        },
        web::{builders::HttpSecurity, configurers::DefaultLoginPageConfigurer},
    },
    core::context::SecurityContextHolderStrategy,
    crypto::{bcrypt::BCryptPasswordEncoder, password::PasswordEncoder},
    web::util::matcher::PathPatternRequestMatcherBuilder,
};

/// Configuration for HTTP security.
pub struct HttpSecurityConfiguration<'a> {
    context: &'a mut dyn ApplicationContext,
    authentication_configuration: AuthenticationConfiguration,
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    content_negotiation_strategy: Arc<dyn ContentNegotiationStrategy>,
}

impl<'a> HttpSecurityConfiguration<'a> {
    fn http_security(&mut self) -> HttpSecurity {
        let password_encoder = Arc::new(SimplePasswordEncoder::from(&*self.context));
        let mut authentication_builder =
            DefaultPasswordEncoderAuthenticationManagerBuilder::new(password_encoder);
        authentication_builder.parent_authentication_manager(self.authentication_manager());
        authentication_builder
            .authentication_event_publisher(self.get_authentication_event_publisher());

        let mut http = HttpSecurity::new(authentication_builder, self.create_shared_objects());

        http.csrf(|_| {})
            .error_handling(|_| {})
            .headers(|_| {})
            .session_management(|_| {})
            .security_context(|_| {})
            .request_cache(|_| {})
            .anonymous(|_| {})
            .with(DefaultLoginPageConfigurer::default(), |_| {});

        http.logout(|_| {});
        self.apply_cors_if_available(&mut http);

        http
    }

    fn apply_cors_if_available(&self, http: &mut HttpSecurity) {
        if self
            .context
            .get_singletons_by_type::<UrlBasedCorsConfigurationSource>()
            .len()
            > 0
        {
            http.cors(|_| {});
        }
    }

    fn create_shared_objects(&self) -> HashMap<TypeId, Box<dyn AnyClone>> {
        let mut shared_objects: HashMap<TypeId, Box<dyn AnyClone>> = HashMap::new();
        shared_objects.insert(
            TypeId::of::<Arc<dyn ContentNegotiationStrategy>>(),
            Box::new(self.content_negotiation_strategy.to_owned()),
        );
        shared_objects.insert(
            TypeId::of::<PathPatternRequestMatcherBuilder>(),
            Box::new(PathPatternRequestMatcherBuilder::default()),
        );

        shared_objects
    }

    fn authentication_manager(&self) -> Arc<dyn AuthenticationManager> {
        self.authentication_configuration
            .get_authentication_manager()
    }

    fn get_authentication_event_publisher(&mut self) -> Arc<dyn AuthenticationEventPublisher> {
        let mut publishers = self
            .context
            .resolve_by_type::<Arc<dyn AuthenticationEventPublisher>>();
        if publishers.len() > 0 {
            publishers.remove(0)
        } else {
            Arc::new(DefaultAuthenticationEventPublisher::default())
        }
    }
}

struct DefaultPasswordEncoderAuthenticationManagerBuilder {
    default_password_encoder: Arc<dyn PasswordEncoder>,
    base: AuthenticationManagerBuilder,
}

impl DefaultPasswordEncoderAuthenticationManagerBuilder {
    /// Creates a new instance
    pub fn new(default_password_encoder: Arc<dyn PasswordEncoder>) -> Self {
        Self {
            default_password_encoder,
            base: AuthenticationManagerBuilder::default(),
        }
    }
}

impl Deref for DefaultPasswordEncoderAuthenticationManagerBuilder {
    type Target = AuthenticationManagerBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for DefaultPasswordEncoderAuthenticationManagerBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

#[derive(Clone)]
struct SimplePasswordEncoder {
    password_encoder: Arc<dyn PasswordEncoder>,
}

impl SimplePasswordEncoder {
    pub fn new(context: &dyn ApplicationContext) -> Self {
        let password_encoder = {
            context
                .get_singleton_option_with_name::<Arc<dyn PasswordEncoder>>("passwordEncoder")
                .map(Clone::clone)
                .unwrap_or(Arc::new(BCryptPasswordEncoder::default()))
        };
        Self { password_encoder }
    }
}

impl PasswordEncoder for SimplePasswordEncoder {
    fn encode(
        &self,
        raw_password: Option<&str>,
    ) -> Result<Option<String>, next_web_core::error::BoxError> {
        self.password_encoder.encode(raw_password)
    }

    fn upgrade_encoding(
        &self,
        encoded_password: &str,
    ) -> Result<bool, next_web_core::error::BoxError> {
        self.password_encoder.upgrade_encoding(encoded_password)
    }

    fn matches(&self, raw_password: &str, encoded_password: &str) -> bool {
        self.password_encoder
            .matches(raw_password, encoded_password)
    }
}
