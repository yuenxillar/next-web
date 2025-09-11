use crate::error::CloneableError;

pub trait RetryState
where
    Self: Send + Sync,
{
    fn get_key(&self) -> Option<&str>;

    fn is_force_refresh(&self) -> bool;

    fn rollback_for(&self, error: &dyn CloneableError) -> bool;
}
