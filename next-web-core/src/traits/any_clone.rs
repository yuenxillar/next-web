use std::{any::Any, fmt};


/// 可克隆的任意类型trait
/// Cloneable any type trait
pub trait AnyClone: Any + Send + Sync {

    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    
    fn clone_box(&self) -> Box<dyn AnyClone>;
}

/// 为所有实现了Clone+Send+Sync的类型实现AnyClone
/// Implement AnyClone for all types that implement Clone+Send+Sync
impl<T> AnyClone for T
where
    T: Any + Clone,
    T: Send + Sync,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for dyn AnyClone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Implement Clone for Box<dyn AnyClone>
impl Clone for Box<dyn AnyClone> {
    fn clone(&self) -> Box<dyn AnyClone> {
        (**self).clone_box()
    }
}