pub trait RoleHierarchy: Send + Sync {
    fn reachable_granted_authorities(&self, authorities: &[String]) -> Vec<String>;
}
