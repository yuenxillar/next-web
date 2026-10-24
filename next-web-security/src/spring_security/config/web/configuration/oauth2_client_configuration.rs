//! Configuration for OAuth 2.0 Client support.
//!
//! The configuration contributes the default authorized client manager of the
//! application, so that an application that describes its client registrations
//! and its authorized clients with repositories can authorize a client without
//! contributing a manager of its own.

use std::{error::Error, sync::Arc};

use next_web_context::ApplicationContextExt;
use next_web_core::{
    async_trait, config::auto_configuration::AutoConfiguration, ApplicationContext, Ordered,
};
use next_web_macros::singleton;

use crate::oauth2::{
    client::{DefaultOAuth2AuthorizedClientManager, OAuth2AuthorizedClientManager},
    ClientRegistrationRepository, OAuth2AuthorizedClientRepository,
};

/// Contributes the default authorized client manager of the application.
///
/// The manager is contributed only when the application did not contribute one
/// itself and describes its client registrations and its authorized clients
/// with exactly one repository each, because a manager has to know both
/// repositories. An application that contributes several of them has to
/// contribute its manager as well.
///
/// The manager is contributed before the filter chains of the web security are
/// built, so that the configurations of the filters can use it.
#[singleton(binds = [Self::into_auto_configuration])]
#[derive(Clone)]
pub struct OAuth2ClientConfiguration;

impl OAuth2ClientConfiguration {
    /// Hands the configuration to the context as an auto-configuration.
    #[must_use]
    pub fn into_auto_configuration(self) -> Box<dyn AutoConfiguration> {
        Box::new(self)
    }
}

#[async_trait]
impl AutoConfiguration for OAuth2ClientConfiguration {
    async fn configure(&mut self, ctx: &mut dyn ApplicationContext) -> Result<(), Box<dyn Error>> {
        if ctx.contains_dyn_singleton_with_default_name::<Arc<dyn OAuth2AuthorizedClientManager>>()
        {
            tracing::debug!(
                "the application registers its own authorized client manager, so the default one \
                 is not installed"
            );
            return Ok(());
        }

        let client_registration_repositories =
            ctx.resolve_by_type::<Arc<dyn ClientRegistrationRepository>>();
        let authorized_client_repositories =
            ctx.resolve_by_type::<Arc<dyn OAuth2AuthorizedClientRepository>>();

        // A manager that picks a repository at random is not useful, so the
        // default manager is contributed only when both are unique.
        if client_registration_repositories.len() != 1 || authorized_client_repositories.len() != 1
        {
            tracing::debug!(
                registrations = client_registration_repositories.len(),
                authorized_clients = authorized_client_repositories.len(),
                "the default authorized client manager is not installed, because the application \
                 does not describe its client registrations and its authorized clients with \
                 exactly one repository each"
            );
            return Ok(());
        }

        let authorized_client_manager = DefaultOAuth2AuthorizedClientManager::new(
            Arc::clone(&client_registration_repositories[0]),
            Arc::clone(&authorized_client_repositories[0]),
        );

        ctx.insert_dyn_singleton_with_default_name::<Arc<dyn OAuth2AuthorizedClientManager>>(
            Arc::new(authorized_client_manager),
        );

        Ok(())
    }

    /// The manager is contributed before the filter chains that use it.
    fn auto_configure_before(&self) -> &'static [&'static str] {
        &["WebSecurityConfiguration"]
    }
}

impl Ordered for OAuth2ClientConfiguration {
    fn order(&self) -> i32 {
        i32::MAX - 20
    }
}
