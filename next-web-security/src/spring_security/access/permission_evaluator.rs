use next_web_core::anys::any_value::AnyValue;

use crate::core::Authentication;

pub trait PermissionEvaluator: Send + Sync {
    fn has_permission(
        &self,
        authentication: &dyn Authentication,
        target: Option<&AnyValue>,
        permission: &str,
    ) -> bool;

    fn has_permission_by_id(
        &self,
        authentication: &dyn Authentication,
        target_id: &str,
        target_type: &str,
        permission: &str,
    ) -> bool;
}
