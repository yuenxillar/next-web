pub trait GrantedAuthority
where
    Self: Send + Sync,
{
    fn authority(&self) -> Option<&str>;
}
