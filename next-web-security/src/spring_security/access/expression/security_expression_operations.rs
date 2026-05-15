use next_web_core::anys::any_value::AnyValue;

pub trait SecurityExpressionOperations {
    fn has_authority(&self, authority: &str) -> bool;
    fn has_any_authority(&self, authorities: &[String]) -> bool;
    fn has_role(&self, role: &str) -> bool;
    fn has_any_role(&self, roles: &[String]) -> bool;
    fn permit_all(&self) -> bool;
    fn deny_all(&self) -> bool;
    fn is_anonymous(&self) -> bool;
    fn is_authenticated(&self) -> bool;
    fn is_remember_me(&self) -> bool;
    fn is_fully_authenticated(&self) -> bool;
    fn has_permission(&self, target: Option<&AnyValue>, permission: &str) -> bool;
    fn has_permission_by_id(&self, target_id: &str, target_type: &str, permission: &str) -> bool;
}
