use std::sync::Arc;

use crate::core::Authentication;

pub trait SecurityContext
where
    Self: Send + Sync,
{
    fn get_authentication(&self) -> Option<Arc<dyn Authentication>>;

    fn set_authentication(&self, authentication: Option<Arc<dyn Authentication>>);
}
