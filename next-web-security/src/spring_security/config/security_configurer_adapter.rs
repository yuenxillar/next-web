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
    composite_object_post_processor: CompositeObjectPostProcessor,

    _marker: std::marker::PhantomData<(O, B)>,
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
        B: Clone,
    {
        None
    }

    pub fn set_builder(&mut self, _security_builder: B) {}
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

impl<O, B> Default for SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
    Self: SecurityConfigurer<O, B>,
{
    fn default() -> Self {
        Self {
            composite_object_post_processor: CompositeObjectPostProcessor {
                post_processors: Vec::new(),
            },
            _marker: std::marker::PhantomData,
        }
    }
}

#[allow(unused_variables)]
impl<O, B> SecurityConfigurer<O, B> for SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
{
    fn init(&mut self, builer: &mut B) {}

    fn configure(&mut self, builer: &mut B) {}
}
