use next_web_core::anys::any_value::AnyValue;

use crate::{
    access::permission_evaluator::PermissionEvaluator,
    core::authentication::Authentication,
};

#[derive(Clone, Debug, Default)]
pub struct DenyAllPermissionEvaluator;

impl PermissionEvaluator for DenyAllPermissionEvaluator {
    fn has_permission(
        &self,
        _authentication: &dyn Authentication,
        _target: Option<&AnyValue>,
        _permission: &str,
    ) -> bool {
        false
    }

    fn has_permission_by_id(
        &self,
        _authentication: &dyn Authentication,
        _target_id: &str,
        _target_type: &str,
        _permission: &str,
    ) -> bool {
        false
    }
}
