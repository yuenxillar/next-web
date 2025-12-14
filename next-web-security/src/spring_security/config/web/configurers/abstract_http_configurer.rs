use std::marker::PhantomData;

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder, security_configurer_adapter::SecurityConfigurerAdapter,
        web::http_security_builder::HttpSecurityBuilder,
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct AbstractHttpConfigurer<T, B>
where
    T: Required<AbstractHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    security_configurer_adapter: SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>,
    _marker_1: PhantomData<T>,
    _marker_2: PhantomData<B>,
}

impl<T, B> AbstractHttpConfigurer<T, B>
where
    T: Required<AbstractHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    pub fn new() -> Self {
        Self {
            security_configurer_adapter: todo!(),
            _marker_1: PhantomData,
            _marker_2: PhantomData,
        }
    }
}

impl<T, B> Default for AbstractHttpConfigurer<T, B>
where
    T: Required<AbstractHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    fn default() -> Self {
        Self {
            security_configurer_adapter: todo!(),
            _marker_1: Default::default(),
            _marker_2: Default::default(),
        }
    }
}

impl<T, B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for AbstractHttpConfigurer<T, B>
where
    T: Required<AbstractHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        &self.security_configurer_adapter
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        &mut self.security_configurer_adapter
    }
}
