use std::{any::Any, sync::Arc};

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
    object_post_processor: CompositeObjectPostProcessor,

    _marker: std::marker::PhantomData<(O, B)>,
}

impl<O, B> SecurityConfigurerAdapter<O, B>
where
    B: SecurityBuilder<O>,
    O: Send + Sync,
    Self: SecurityConfigurer<O, B>,
{
    pub fn post_process(&mut self, object: &mut dyn Any) {
        self.object_post_processor.post_process(object);
    }

    pub fn get_builder(&mut self) -> Option<&mut B>
    where
        B: Clone,
    {
        None
    }

    pub fn set_builder(&mut self, _security_builder: B) {}
}

#[derive(Clone)]
pub struct CompositeObjectPostProcessor {
    post_processors: Vec<Arc<dyn ObjectPostProcessor<dyn Any>>>,
}

impl CompositeObjectPostProcessor {
    fn add_object_post_processor<T>(&mut self, object_post_processor: T)
    where
        T: ObjectPostProcessor<dyn Any>,
    {
        // self.post_processors.push(Box::new(object_post_processor));

        todo!()
    }
}

impl ObjectPostProcessor<dyn Any> for CompositeObjectPostProcessor {
    fn post_process(&self, object: &mut dyn Any) {
        for opp in self.post_processors.iter() {
            opp.post_process(object);
        }
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
            object_post_processor: CompositeObjectPostProcessor {
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
    fn init(&mut self, builder: &mut B) {}

    fn configure(&mut self, builder: &mut B) {}
}
