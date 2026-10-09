//! Cloneable, type-erased values.
//!
//! [`AnyClone`] pairs [`std::any::Any`] with cloning so that heterogeneous
//! values can be stored and duplicated through a `dyn` trait object. It is the
//! storage primitive behind [`crate::any::AnyValue::BoxedValue`].

use std::{any::Any, fmt};

/// A type-erased value that can be cloned through a trait object.
///
/// Rust's [`Any`] trait lets a value be downcast but is not `Clone`, while
/// [`Clone`] is not object safe. `AnyClone` closes that gap: a
/// `Box<dyn AnyClone>` owns a value of an unknown concrete type, can be cloned,
/// and lets the original type be recovered with [`AnyClone::as_any`] or
/// [`AnyClone::into_any`].
///
/// A blanket implementation covers every `T: Any + Clone + Send + Sync`, so
/// manual implementations are rarely needed.
///
/// # Examples
///
/// ```
/// use next_web_core::any::AnyClone;
///
/// let original: Box<dyn AnyClone> = Box::new(42_i32);
///
/// // `Box<dyn AnyClone>` also satisfies the blanket impl, so reach the erased
/// // value itself with `&*original` before calling the trait methods.
/// let cloned = AnyClone::clone_box(&*original);
/// let restored = AnyClone::as_any(&*cloned);
///
/// assert_eq!(restored.downcast_ref::<i32>(), Some(&42));
/// ```
pub trait AnyClone: Any + Send + Sync {
    /// Returns this value as `&dyn Any` so it can be downcast to its concrete
    /// type.
    fn as_any(&self) -> &dyn Any;

    /// Consumes the boxed value and returns it as `Box<dyn Any>`.
    ///
    /// This is allocation free and lets the caller recover the owned concrete
    /// value without cloning it.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Clones the value into a new, boxed trait object.
    ///
    /// The blanket implementation simply boxes `self.clone()`. Implementors
    /// that can clone more cheaply may override it.
    fn clone_box(&self) -> Box<dyn AnyClone>;
}

/// Implements [`AnyClone`] for every clonable, thread-safe `'static` type.
impl<T> AnyClone for T
where
    T: Any + Clone,
    T: Send + Sync,
{
    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn clone_box(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for dyn AnyClone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The concrete value is erased, so its own `Debug` output is
        // unavailable here. Printing the `TypeId` keeps the output informative
        // and, unlike a naive `write!(f, "{self:?}")`, does not recurse back
        // into this implementation.
        f.debug_struct("dyn AnyClone")
            .field("type_id", &self.as_any().type_id())
            .finish()
    }
}

impl Clone for Box<dyn AnyClone> {
    #[inline]
    fn clone(&self) -> Box<dyn AnyClone> {
        (**self).clone_box()
    }
}
