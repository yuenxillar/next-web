use std::{any::Any, sync::Arc};

use crate::core::{Authentication, AuthenticationError};

pub trait AuthenticationManager
where
    Self: Send + Sync,
    Self: Any,
{
    fn authenticate(
        &self,
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError>;
}
