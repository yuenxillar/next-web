pub trait RoleHierarchy: Send + Sync {
    fn get_reachable_granted_authorities(&self, authorities: &[String]) -> Vec<String>;
}
