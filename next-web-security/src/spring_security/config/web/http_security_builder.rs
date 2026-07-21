use std::sync::Arc;

use next_web_core::traits::{any_clone::AnyClone, filter::HttpFilter};

use crate::{
    authentication::AuthenticationProvider,
    config::{security_builder::SecurityBuilder, security_configurer::SecurityConfigurer},
    core::userdetails::UserDetailsService,
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

pub trait HttpSecurityBuilder<H>
where
    Self: Send + Sync,
    Self: SecurityBuilder<DefaultSecurityFilterChain>,
    H: HttpSecurityBuilder<H>,
{
    fn configurer<C>(&self) -> Option<&C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, H>,
        C: 'static;

    fn configurer_mut<C>(&mut self) -> Option<&mut C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, H>,
        C: 'static;

    fn remove_configurer<C>(&mut self) -> Option<C>
    where
        C: SecurityConfigurer<DefaultSecurityFilterChain, H>,
        C: AnyClone + 'static;

    fn set_shared_object<C>(&mut self, object: C)
    where
        C: AnyClone;

    fn shared_object<T>(&self) -> Option<&T>
    where
        T: AnyClone;

    fn shared_object_mut<T>(&mut self) -> Option<&mut T>
    where
        T: AnyClone;

    fn authentication_provider(&mut self, authentication_provider: Arc<dyn AuthenticationProvider>);

    fn user_details_service<T>(&mut self, user_details_service: T)
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
}
