use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::filter::FilterError;

use crate::core::context::security_context::SecurityContext;

pub trait SecurityContextHolderStrategy
where
    Self: Send + Sync,
{
    fn clear_context(&self);

    fn get_context(&self) -> Option<Arc<dyn SecurityContext>>;

    fn set_context(&self, context: Arc<dyn SecurityContext>);

    fn scope_with_context<'a>(
        &'a self,
        context: Arc<dyn SecurityContext>,
        f: BoxFuture<'a, Result<(), FilterError>>,
    ) -> BoxFuture<'a, Result<(), FilterError>>;

    fn create_empty_context(&self) -> Arc<dyn SecurityContext>;
}
