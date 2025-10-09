use std::sync::Arc;

use next_web_core::{constants::application_constants::SECURE_PROPERTIES_MARK, models::any_value::AnyValue, traits::any_clone::AnyClone};

use crate::config::object_post_processor::ObjectPostProcessor;


pub struct SecurityConfigurerAdapter<B> {

    security_builder: B,
    composite_object_post_processor: CompositeObjectPostProcessor,

}

impl<B> SecurityConfigurerAdapter<B> {

    pub fn post_process<T>(
        &self,
        object: T
    ) 
    where T: AnyClone
    {
        self.composite_object_post_processor.post_process(AnyValue::Object(Box::new(object)));
    }
}

pub struct CompositeObjectPostProcessor {
    post_processors: Vec<Arc<dyn ObjectPostProcessor>>,
}

impl CompositeObjectPostProcessor  {
    fn add_object_post_processor(&mut self, var: impl ObjectPostProcessor + 'static){
        self.post_processors.push(Arc::new(var));
    }
}

impl ObjectPostProcessor for CompositeObjectPostProcessor {

    fn  post_process(&self, mut object: AnyValue) -> AnyValue {
        for opp in self.post_processors.iter().map(AsRef::as_ref) {
            object = opp.post_process(object);
        }

        object
    }
}