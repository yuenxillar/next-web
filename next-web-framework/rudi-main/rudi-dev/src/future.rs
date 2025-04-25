use std::{future::Future, pin::Pin};

/// An owned dynamically typed [`Future`] for use in cases where you can't
/// statically type your result or need to add some indirection.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

impl<T: ?Sized + Send + Sync> FutureExt for T where T: Future {}

/// An extension trait for `Future`s that provides a convenient adapter.
pub trait FutureExt: Future {
    /// Wrap the future in a Box, pinning it.
    fn boxed<'a>(self) -> BoxFuture<'a, Self::Output>
    where
        Self: Sized + 'static + Send + Sync,
    {
        Box::pin(self)
    }
}
