use std::{any::Any, sync::Arc};

/// Represents a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance.
pub struct Single<T> {
    instance: T,
    clone: Option<fn(&T) -> T>,
}

impl<T> Single<T> {
    pub(crate) fn new(instance: T, clone: Option<fn(&T) -> T>) -> Self {
        Self { instance, clone }
    }

    /// Returns the owned instance.
    pub fn get_owned(&self) -> Option<T> {
        self.clone.map(|clone| clone(&self.instance))
    }

    /// Returns a reference to the instance.
    pub fn get_ref(&self) -> &T {
        &self.instance
    }
}

/// Represents a [`Single`] that erased its type.

pub struct DynSingle {
    origin: Arc<dyn Any + Send + Sync>,
}

impl Clone for DynSingle {
    
    fn clone(&self) -> Self {
        Self {
            origin: Arc::new(self.origin.clone()),
        }
    }
}


impl DynSingle {
    /// Returns a reference of the origin [`Single`].
    pub fn as_single<T: 'static>(&self) -> Option<&Single<T>> {
        self.origin.downcast_ref::<Single<T>>()
    }
}

impl<T: 'static + Send + Sync> From<Single<T>> for DynSingle {
    fn from(value: Single<T>) -> Self {
        Self {
            origin: Arc::new(value),
        }
    }
}
