//! The authentication manager of the application.

use std::sync::Arc;

use crate::{
    authentication::provider_manager::ProviderManager, authorization::AuthenticationManager,
};

/// Holds the [`AuthenticationManager`] the security filter chains authenticate
/// their requests with.
///
/// The manager of the application is the manager the filter chains were built
/// with, which knows every provider the application registered. A configuration
/// that was not given a manager holds one without a provider, so a request it
/// authenticates is reported as not authenticatable instead of the request
/// failing the application.
#[derive(Clone)]
pub struct AuthenticationConfiguration {
    authentication_manager: Arc<dyn AuthenticationManager>,
}

impl AuthenticationConfiguration {
    /// Creates a configuration that holds the given manager.
    ///
    /// # Arguments
    ///
    /// * `authentication_manager` - The manager of the application.
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager,
        }
    }

    /// Sets the manager the configuration holds.
    ///
    /// # Arguments
    ///
    /// * `authentication_manager` - The manager of the application.
    pub fn set_authentication_manager(
        &mut self,
        authentication_manager: Arc<dyn AuthenticationManager>,
    ) {
        self.authentication_manager = authentication_manager;
    }

    /// Returns the manager of the application.
    pub fn get_authentication_manager(&self) -> Arc<dyn AuthenticationManager> {
        self.authentication_manager.clone()
    }
}

impl Default for AuthenticationConfiguration {
    fn default() -> Self {
        Self {
            authentication_manager: Arc::new(ProviderManager::new(Vec::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::AuthenticationConfiguration;
    use crate::{
        authentication::{provider_manager::ProviderManager, UsernamePasswordAuthenticationToken},
        authorization::AuthenticationManager,
        core::AuthenticationErrorKind,
    };

    #[tokio::test]
    async fn a_configuration_without_a_manager_reports_that_no_provider_can_authenticate() {
        let configuration = AuthenticationConfiguration::default();
        let manager = configuration.get_authentication_manager();
        let token = UsernamePasswordAuthenticationToken::unauthenticated(
            Some(Arc::new("user".to_string())),
            Some(Arc::new("password".to_string())),
        );

        let error = manager
            .authenticate(&token)
            .await
            .expect_err("a manager without a provider cannot authenticate the request");

        assert_eq!(error.kind(), AuthenticationErrorKind::ProviderNotFound);
    }

    #[test]
    fn a_configuration_holds_the_manager_it_was_given() {
        let manager: Arc<dyn AuthenticationManager> = Arc::new(ProviderManager::new(Vec::new()));
        let configuration = AuthenticationConfiguration::new(Arc::clone(&manager));

        assert!(Arc::ptr_eq(
            &configuration.get_authentication_manager(),
            &manager
        ));
    }
}
