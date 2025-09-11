use std::error::Error;

use crate::retry_context::RetryContext;



pub trait RecoveryCallback<T>
where
    Self: Send + Sync,
     {
    
    fn recover(&self, context: &dyn RetryContext) -> Result<T, Box<dyn Error>>;
}