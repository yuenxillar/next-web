pub trait ObjectPostProcessor<T>
where
    T: ?Sized,
{
    fn post_process(&mut self, object: &mut T);
}
