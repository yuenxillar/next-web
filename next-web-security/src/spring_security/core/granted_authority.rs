use std::any::Any;

pub trait GrantedAuthority
where
    Self: Send + Sync,
    Self: Any,
{
    fn authority(&self) -> Option<&str>;
}
