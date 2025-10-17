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
    O: Send + Sync,
    Self: SecurityConfigurer<O, B>,
{
    security_builder: Option<B>,
    composite_object_post_processor: CompositeObjectPostProcessor,

    _marker: std::marker::PhantomData<O>,
}

impl<O, B> SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
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
    post_processors: Vec<Arc<dyn ObjectPostProcessor<AnyValue>>>,
}

impl CompositeObjectPostProcessor {
    fn add_object_post_processor(&mut self, var: impl ObjectPostProcessor<AnyValue> + 'static) {
        self.post_processors.push(Arc::new(var));
    }
}

impl ObjectPostProcessor<AnyValue> for CompositeObjectPostProcessor {
    fn post_process(&self, object: AnyValue) -> Option<AnyValue> {
        let mut value = Some(object);
        for opp in self.post_processors.iter().map(AsRef::as_ref) {
            if let Some(val) = value {
                value = opp.post_process(val);
            }
        }
        value
    }
}

#[allow(unused_variables)]
impl<O, B> SecurityConfigurer<O, B> for SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync
{
    fn init(&mut self, builer: &mut B) {}

    fn configure(&mut self, builer: &mut B) {}
}
