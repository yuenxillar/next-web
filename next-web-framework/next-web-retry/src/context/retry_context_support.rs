use crate::retry_context::{AttributeAccessor, RetryContext};


#[derive(Clone)]
pub struct RetryContextSupport {

}

impl RetryContextSupport {
    pub fn new() -> Self {
        Self {}
    }


    pub fn register_error(&mut self, error: Option<&dyn crate::error::AnyError>) {

    }
}

impl AttributeAccessor for RetryContextSupport {
    fn has_attribute(&self, name: &str) -> bool {
        todo!()
    }

    fn set_attribute(&mut self, name: &str, value: next_web_core::util::any_map::AnyValue) {
        todo!()
    }

    fn remove_attribute(&mut self, name: &str) -> Option<next_web_core::util::any_map::AnyValue> {
        todo!()
    }

    fn get_attribute(&self, name: &str) -> Option<& next_web_core::util::any_map::AnyValue> {
        todo!()
    }
}

impl RetryContext for RetryContextSupport {
    fn set_exhausted_only(&mut self) {
        todo!()
    }

    fn is_exhausted_only(&self) -> bool {
        todo!()
    }

    fn get_parent(&self) -> Option<&dyn RetryContext> {
        todo!()
    }

    fn get_retry_count(&self) -> u16 {
        todo!()
    }

    fn get_last_error(&self) -> Option<Box<dyn crate::error::AnyError>> {
        todo!()
    }
}