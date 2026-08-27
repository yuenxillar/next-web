use std::{marker::PhantomData, sync::Arc};

use next_web_core::traits::required::Required;

use crate::{
    config::{
        security_builder::SecurityBuilder, security_configurer_adapter::SecurityConfigurerAdapter,
        web::http_security_builder::HttpSecurityBuilder,
    },
    core::context::{SecurityContextHolder, SecurityContextHolderStrategy},
    web::default_security_filter_chain::DefaultSecurityFilterChain,
};

#[derive(Clone)]
pub struct BaseHttpConfigurer<T, B>
where
    B: HttpSecurityBuilder<B>,
{
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    base: SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>,
    _marker: PhantomData<T>,
}

impl<T, B> BaseHttpConfigurer<T, B>
where
    B: HttpSecurityBuilder<B>,
{
    pub fn get_security_context_holder_strategy(&self) -> &Arc<dyn SecurityContextHolderStrategy> {
        // TODO!(Extract singleton from application context)
        &self.security_context_holder_strategy
    }
}

impl<T, B> Default for BaseHttpConfigurer<T, B>
where
    B: HttpSecurityBuilder<B>,
{
    fn default() -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),

            base: SecurityConfigurerAdapter::default(),
            _marker: Default::default(),
        }
    }
}

impl<T, B> Required<SecurityConfigurerAdapter<DefaultSecurityFilterChain, B>>
    for BaseHttpConfigurer<T, B>
where
    B: HttpSecurityBuilder<B>,
    B: SecurityBuilder<DefaultSecurityFilterChain>,
{
    fn get_object(&self) -> &SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        &self.base
    }

    fn get_mut_object(&mut self) -> &mut SecurityConfigurerAdapter<DefaultSecurityFilterChain, B> {
        &mut self.base
    }
}

// pub trait BaseHttpConfigurerExt<H>
// where
//     H: HttpSecurityBuilder<H>,
// {
//     fn disable(&mut self, http: &mut H);
// }

// impl<T, H> BaseHttpConfigurerExt<H> for T
// where
//     H: HttpSecurityBuilder<H>,
//     T: SecurityConfigurer<DefaultSecurityFilterChain, H>,
//     T: Sized + 'static,
// {
//     fn disable(&mut self, http: &mut H) {
//         let _ = http.remove_configurer::<Self>();
//     }
// }
