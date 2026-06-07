use std::sync::Arc;

use futures::future::BoxFuture;
use next_web_core::error::BoxError;
use tokio::task_local;

use crate::core::context::{
    security_context::SecurityContext,
    security_context_holder_strategy::SecurityContextHolderStrategy, SecurityContextImpl,
};

task_local! {
    static CONTEXT_HOLDER: Arc<dyn SecurityContext>
}

#[derive(Clone, Default)]
pub struct ThreadLocalSecurityContextHolderStrategy;

impl SecurityContextHolderStrategy for ThreadLocalSecurityContextHolderStrategy {
    fn clear_context(&self) {
        unimplemented!()
    }

    fn get_context(&self) -> Option<Arc<dyn SecurityContext>> {
        CONTEXT_HOLDER.try_get().ok()
    }

    fn scope_with_context<'a>(
        &'a self,
        context: Arc<dyn SecurityContext>,
        f: BoxFuture<'a, Result<(), BoxError>>,
    ) -> BoxFuture<'a, Result<(), BoxError>> {
        Box::pin(CONTEXT_HOLDER.scope(context, async move { f.await }))
    }

    fn set_context(&self, context: Arc<dyn SecurityContext>) {
        unimplemented!()
    }

    fn create_empty_context(&self) -> Arc<dyn SecurityContext> {
        Arc::new(SecurityContextImpl::default())
    }
}
