/// Finds additional required authorities for the provided Authentication.name()
pub trait RequiredAuthoritiesRepository: Send + Sync {
    /// Finds additional required GrantedAuthority.authority()s for the provided username.
    fn find_required_authorities(&self, username: &str) -> Vec<&str>;
}
