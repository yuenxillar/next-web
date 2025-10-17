use next_web_core::traits::required::Required;
use std::sync::Arc;

use crate::authentication::authentication_provider::AuthenticationProvider;
use crate::config::abstract_configured_security_builder::AbstractConfiguredSecurityBuilder;
use crate::core::userdetails::user_details_service::UserDetailsService;
use crate::{
    authorization::authentication_manager::AuthenticationManager,
    config::{
        authentication::provider_manager_builder::ProviderManagerBuilder,
        security_builder::SecurityBuilder,
    },
};

#[derive(Clone)]
pub struct AuthenticationManagerBuilder
where
    Self: Required<AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>>,
{
    authentication_manager: Option<Arc<dyn AuthenticationManager>>,
    authentication_providers: Vec<Arc<dyn AuthenticationProvider>>,
    default_user_details_service: Option<Arc<dyn UserDetailsService>>,
    erase_credentials: Option<bool>,

    abstract_configured_security_builder:
        AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>,
}

impl AuthenticationManagerBuilder {
    pub fn new() -> Self {
        let abstract_configured_security_builder = AbstractConfiguredSecurityBuilder::new(true);
        Self {
            authentication_manager: Default::default(),
            authentication_providers: Default::default(),
            default_user_details_service: Default::default(),
            erase_credentials: Default::default(),
            abstract_configured_security_builder,
        }
    }
}

impl ProviderManagerBuilder<Self> for AuthenticationManagerBuilder {}

impl SecurityBuilder<Arc<dyn AuthenticationManager>> for AuthenticationManagerBuilder {
    fn build(&self) -> Arc<dyn AuthenticationManager> {
        todo!()
    }
}

impl Required<AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self>>
    for AuthenticationManagerBuilder
{
    fn get_object(
        &self,
    ) -> &AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &self.abstract_configured_security_builder
    }

    fn get_object_mut(
        &mut self,
    ) -> &mut AbstractConfiguredSecurityBuilder<Arc<dyn AuthenticationManager>, Self> {
        &mut self.abstract_configured_security_builder
    }
}
