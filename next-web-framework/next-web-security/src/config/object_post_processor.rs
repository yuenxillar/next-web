use next_web_core::models::any_value::AnyValue;



pub trait ObjectPostProcessor: Send +Sync {
    
    fn  post_process(&self, object: AnyValue) -> AnyValue;
}