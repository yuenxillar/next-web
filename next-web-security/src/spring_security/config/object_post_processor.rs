pub trait ObjectPostProcessor<T>
where
    T: ?Sized,
    Self: Send + Sync,
{
    fn post_process(&self, object: &mut T);
}
