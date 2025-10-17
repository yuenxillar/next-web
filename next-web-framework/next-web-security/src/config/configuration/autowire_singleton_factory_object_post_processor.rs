use next_web_core::anys::any_value::AnyValue;

use crate::config::object_post_processor::ObjectPostProcessor;



#[derive(Clone)]
pub struct AutowireSingletonFactoryObjectPostProcessor {
    
}

impl ObjectPostProcessor<AnyValue> for AutowireSingletonFactoryObjectPostProcessor {
    fn  post_process(&self, object: AnyValue) -> Option<AnyValue> {
        let result = None;


        result
    }
}