use crate::{error::AnyError, retry_callback::RetryCallback, retry_context::RetryContext};

pub trait RetryListener
where
    Self: Send + Sync,
{
    // fn open<T, E: AnyError>(&self, context: &dyn RetryContext, call_back: &dyn RetryCallback<T, E>) -> bool;

    fn on_error(
        &self,
        context: &dyn RetryContext,
        callback: &dyn RetryCallback<String>,
        error: &dyn AnyError,
    ) {
    }

    fn on_success(
        &self,
        context: &dyn RetryContext,
        callback: &dyn RetryCallback<String>,
        result: & String,
    ) {
    }
}

pub struct DefaultRetryListener {}

impl RetryListener for DefaultRetryListener {

    // fn open<T, E: AnyError>(&self, context: &dyn RetryContext, call_back: &dyn RetryCallback<T, E>) -> bool {
    //     true
    // }
}
