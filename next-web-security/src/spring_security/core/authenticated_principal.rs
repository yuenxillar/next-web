pub trait AuthenticatedPrincipal
where
    Self: Send + Sync,
{
    fn name(&self) -> &str;
}
