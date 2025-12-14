use std::borrow::Cow;

use next_web_core::traits::any_clone::AnyClone;

use crate::{
    config::{security_builder::SecurityBuilder, security_configurer::SecurityConfigurer},
    core::filter::Filter,
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

    fn get_shared_object<T>(&self) -> Option<&T>;

    fn get_mut_shared_object<T>(&mut self) -> Option<&mut T>;

    fn add_filter<F: Filter>(self, filter: F) -> Self;

    fn add_filter_after<F, F1>(self, filter: F, after_filter: F1) -> Self
    where
        F: Filter,
        F1: Filter;

    fn add_filter_before<F, F1>(self, filter: F, before_filter: F1) -> Self
    where
        F: Filter,
        F1: Filter;

    fn set_shared_object<N, C>(&self, name: N, object: C)
    where
        N: Into<Cow<'static, str>>,
        C: AnyClone;
}
