use crate::core::context::security_context::SecurityContext;

pub trait SecurityContextHolderStrategy: Send + Sync {
    fn clear_context(&self);

    fn get_context(&self) -> SecurityContext;

    fn set_context(&self, context: SecurityContext);

    fn create_empty_context(&self) -> SecurityContext {
        SecurityContext::default()
    }
}
