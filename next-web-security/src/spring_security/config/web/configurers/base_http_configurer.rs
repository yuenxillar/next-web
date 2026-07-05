use std::{marker::PhantomData, sync::Arc};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder, security_configurer_adapter::SecurityConfigurerAdapter,
        web::http_security_builder::HttpSecurityBuilder,
    },
    core::context::{
        security_context_holder::SecurityContextHolder,
        security_context_holder_strategy::SecurityContextHolderStrategy,
    },
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct BaseHttpConfigurer<T, B>
where
    T: Required<BaseHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    security_configurer_adapter: SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>,
    _marker_1: PhantomData<T>,
}

impl<T, B> BaseHttpConfigurer<T, B>
where
    T: Required<BaseHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    pub fn get_security_context_holder_strategy(&self) -> &Arc<dyn SecurityContextHolderStrategy> {
        &self.security_context_holder_strategy
    }
}

impl<T, B> Default for BaseHttpConfigurer<T, B>
where
    T: Required<BaseHttpConfigurer<T, B>>,
    B: HttpSecurityBuilder<B>,
    Self: Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>,
{
    fn default() -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            security_configurer_adapter: SecurityConfigurerAdapter::default(),
            _marker_1: Default::default(),
        }
    }
}

impl<T, B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for BaseHttpConfigurer<T, B>
where
    T: Required<BaseHttpConfigurer<T, B>>,
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

pub trait BaseHttpConfigurerExt<B> {
    fn disable(&mut self) -> B;
}
