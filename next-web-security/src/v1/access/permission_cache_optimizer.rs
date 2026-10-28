use next_web_core::any::AnyValue;

use crate::core::Authentication;

pub trait PermissionCacheOptimizer: Send + Sync {
    fn cache_permissions_for(&self, authentication: &dyn Authentication, objects: &[AnyValue]);
}

#[derive(Clone, Debug, Default)]
pub struct NoOpPermissionCacheOptimizer;

impl PermissionCacheOptimizer for NoOpPermissionCacheOptimizer {
    fn cache_permissions_for(&self, _authentication: &dyn Authentication, _objects: &[AnyValue]) {}
}
