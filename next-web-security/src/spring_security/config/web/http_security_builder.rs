use std::{borrow::Cow, sync::Arc};

use next_web_core::traits::{any_clone::AnyClone, filter::HttpFilter};

use crate::{
    authentication::authentication_provider::AuthenticationProvider,
    authorization::AuthenticationManager,
    config::{security_builder::SecurityBuilder, security_configurer::SecurityConfigurer},
    core::userdetails::user_details_service::UserDetailsService,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

pub trait HttpSecurityBuilder<H>
where
    Self: Send + Sync,
    Self: SecurityBuilder<DefaultSecurityFilterChain>,
    H: HttpSecurityBuilder<H>,
{
    fn get_configurer<T>(&self) -> Option<T>
    where
        T: SecurityConfigurer<DefaultSecurityFilterChain, H>;

    fn remove_configurer<T>(&mut self)
    where
        T: SecurityConfigurer<DefaultSecurityFilterChain, H>;

    fn set_shared_object<N, C>(&self, name: N, object: C)
    where
        N: Into<Cow<'static, str>>,
        C: AnyClone;

    fn get_shared_object<T>(&self) -> Option<&T>;

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T>;

    fn authentication_provider<T>(&mut self, authentication_provider: T) -> H
    where
        T: AuthenticationProvider + 'static;

    fn user_details_service<T>(&mut self, user_details_service: T) -> H
    where
        T: UserDetailsService + 'static;

    fn add_filter<F: HttpFilter>(&mut self, filter: F);

    fn add_filter_after<F, F1>(&mut self, filter: F)
    where
        F: HttpFilter,
        F1: HttpFilter;

    fn add_filter_before<F, F1>(&mut self, filter: F)
    where
        F: HttpFilter,
        F1: HttpFilter;

    /// Returns the configured `AuthenticationManager`, if any.
    fn authentication_manager(&self) -> Option<Arc<dyn AuthenticationManager>> {
        None
    }
}
