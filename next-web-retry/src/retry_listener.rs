use std::any::Any;

use next_web_core::anys::any_error::AnyError;

use crate:: retry_context::RetryContext;

pub trait RetryListener
where
    Self: Send + Sync,
{
    fn open(&self, context: &dyn RetryContext) -> bool { true }

    fn close(&self, context: &dyn RetryContext, error: Option<&dyn AnyError>) {}

    fn on_success(&self, context: &dyn RetryContext, result: &dyn Any) {}

    fn on_error(&self, context: &dyn RetryContext, error: &dyn AnyError) {}
}


pub struct DefaultRetryListener {}

impl RetryListener for DefaultRetryListener {
    // fn open<T, E: AnyError>(&self, context: &dyn RetryContext, call_back: &dyn RetryCallback<T, E>) -> bool {
    //     true
    // }
}
