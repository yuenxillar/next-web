/// Policy that decides whether a given location is allowed to be loaded.
pub trait ResourceAccessPolicy
where
    Self: Send + Sync,
{
    /// Return `true` if the location is permitted.
    fn is_allowed(&self, location: &str) -> bool;
}
