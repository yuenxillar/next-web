use std::{
    sync::{Arc, LazyLock},
    vec,
};

use crate::util::Regex;
use next_web_context::{APPLICATION_ENVIRONMENT_SINGLETON_NAME, ApplicationContextExt};
use next_web_core::{ApplicationContext, env::ConfigurableEnvironment};
use next_web_macros::auto_configuration;
use next_web_security::{
    authentication::AuthenticationProvider,
    authorization::AuthenticationManager,
    core::userdetails::{User, UserDetailsService},
    crypto::password::PasswordEncoder,
    provisioning::{InMemoryUserDetailsManager, UserDetailsManager},
};

use crate::autoconfigure::security::SecurityProperties;

static PASSWORD_ALGORITHM_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\{.+}.*$").unwrap());

const NOOP_PASSWORD_PREFIX: &str = "{noop}";

/// Auto-configuration for a Web Security in-memory AuthenticationManager.
/// Adds an InMemoryUserDetailsManager with a default user and generated password.
#[derive(Debug, Clone)]
pub struct UserDetailsServiceAutoConfiguration;

#[auto_configuration(conditional = [Self::on_missing_singleton, Self::missing_alternative_user_details_manager_or_user_properties_configured])]
impl UserDetailsServiceAutoConfiguration {
    #[provider(name = "userDetailsManager")]
    fn in_memory_user_details_manager(
        security_properties: SecurityProperties,
        #[autowired(default)] password_encoder: Option<Arc<dyn PasswordEncoder>>,
    ) -> Arc<dyn UserDetailsManager> {
        let user = security_properties.user();
        let roles = user.roles();

        Arc::new(InMemoryUserDetailsManager::with_users(vec![Arc::new(
            User::with_username(user.name())
                .password(
                    UserDetailsServiceAutoConfiguration::get_or_deduce_password(
                        &security_properties,
                        password_encoder.as_deref(),
                    )
                    .into(),
                )
                .roles(roles)
                .build(),
        )]))
    }

    fn on_missing_singleton(ctx: &dyn ApplicationContext) -> bool {
        !ctx.contains_dyn_singleton_with_default_name::<Arc<dyn AuthenticationManager>>()
            && !ctx.contains_dyn_singleton_with_default_name::<Arc<dyn AuthenticationProvider>>()
            && !ctx.contains_dyn_singleton_with_default_name::<Arc<dyn UserDetailsService>>()
    }

    fn missing_alternative_user_details_manager_or_user_properties_configured(
        ctx: &dyn ApplicationContext,
    ) -> bool {
        let env = ctx.get_singleton_with_name::<Arc<dyn ConfigurableEnvironment>>(
            APPLICATION_ENVIRONMENT_SINGLETON_NAME,
        );

        env.contains_property("next.security.user.name")
            && env.contains_property("next.security.user.password")
    }

    fn get_or_deduce_password(
        security_properties: &SecurityProperties,
        encoder: Option<&dyn PasswordEncoder>,
    ) -> String {
        let user = security_properties.user();
        let password = user.password();
        if user.is_password_generated() {
            tracing::warn!(
                "\nUsing generated security password: {password}\nThis generated password is for development use only.
						Your security configuration must be updated before running your application in
						production.\n",
				);
        }

        if encoder.is_some() || PASSWORD_ALGORITHM_PATTERN.is_match(password) {
            return password.to_owned();
        }

        format!("{NOOP_PASSWORD_PREFIX}{password}")
    }
}
