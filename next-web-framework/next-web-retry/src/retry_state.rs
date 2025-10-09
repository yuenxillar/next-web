use next_web_core::models::any_error::AnyError;


pub trait RetryState<K = String>
where
    Self: Send + Sync,
{
    fn get_key(&self) -> Option<&K>;

    fn is_force_refresh(&self) -> bool;

    fn rollback_for(&self, error: &dyn AnyError) -> bool;
}
