use std::sync::Arc;

use next_web_core::{anys::any_value::AnyValue, traits::any_clone::AnyClone};

use crate::config::{
    object_post_processor::ObjectPostProcessor, security_builder::SecurityBuilder,
    security_configurer::SecurityConfigurer,
};

#[derive(Clone)]
pub struct SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    Self: SecurityConfigurer<O, B>,
{
    security_builder: Option<B>,
    composite_object_post_processor: CompositeObjectPostProcessor,

    _marker: std::marker::PhantomData<O>,
}

impl<O, B> SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    Self: SecurityConfigurer<O, B>,
{
    pub fn post_process<T>(&self, object: T)
    where
        T: AnyClone,
    {
        self.composite_object_post_processor
            .post_process(AnyValue::Object(Box::new(object)));
    }

    
    pub fn get_builder(&self) -> Option<B> 
    where 
    B: Clone
    {
        assert!(self.security_builder.is_some(), "security_builder cannot be null");
        self.security_builder.clone()
    }
}

#[derive(Clone)]
pub struct CompositeObjectPostProcessor {
    post_processors: Vec<Arc<dyn ObjectPostProcessor>>,
}

impl CompositeObjectPostProcessor {
    fn add_object_post_processor(&mut self, var: impl ObjectPostProcessor + 'static) {
        self.post_processors.push(Arc::new(var));
    }
}

impl ObjectPostProcessor for CompositeObjectPostProcessor {
    fn post_process(&self, mut object: AnyValue) -> AnyValue {
        for opp in self.post_processors.iter().map(AsRef::as_ref) {
            object = opp.post_process(object);
        }

        object
    }
}

impl<O, B> SecurityConfigurer<O, B> for SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
{
    fn init(&self, builer: B) {
        todo!()
    }

    fn configure(&self, builer: B) {
        todo!()
    }

}
