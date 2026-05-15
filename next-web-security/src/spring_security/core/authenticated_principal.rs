pub trait AuthenticatedPrincipal: Send + Sync {
    fn name(&self) -> &str;
}
