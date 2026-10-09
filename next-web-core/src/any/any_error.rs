//! Cloneable, hashable, type-erased errors.
//!
//! [`AnyError`] is the error analogue of [`crate::any::AnyClone`]: it lets an
//! owned error be stored behind `Box<dyn AnyError>` while remaining clonable,
//! downcastable, and usable as a hash-map key.

use std::{
    any::Any,
    hash::{Hash, Hasher},
    ptr,
};

use dyn_clone::DynClone;

/// A boxed, clonable, hashable error object.
///
/// The standard [`std::error::Error`] trait does not, on stable Rust, expose
/// cloning, downcasting, or hashing through a trait object. `AnyError` adds
/// exactly those capabilities so errors can flow through type-erased channels:
///
/// * [`AnyError::to_boxed`] clones the error,
/// * [`AnyError::as_any`] downcasts it back to a concrete type,
/// * [`AnyError::hash_dyn`] feeds it into a type-erased [`Hasher`].
///
/// A blanket implementation provides all three for every
/// `T: Error + Clone + Send + Sync + Hash + 'static`.
pub trait AnyError
where
    Self: Send + Sync,
    Self: std::error::Error + DynClone,
{
    /// Clones this error into a new boxed trait object.
    fn to_boxed(&self) -> Box<dyn AnyError>;

    /// Returns this error as `&dyn Any` so it can be downcast to its concrete
    /// type.
    fn as_any(&self) -> &dyn Any;

    /// Hashes the concrete error value into `state`.
    ///
    /// [`Hash::hash`] is generic over a sized hasher, so the erased error
    /// forwards its bytes through the supplied `&mut dyn Hasher`.
    fn hash_dyn(&self, state: &mut dyn Hasher);
}

impl<T> AnyError for T
where
    T: std::error::Error + Clone,
    T: Send + Sync + 'static,
    T: Hash,
{
    #[inline]
    fn to_boxed(&self) -> Box<dyn AnyError> {
        Box::new(self.clone())
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn hash_dyn(&self, state: &mut dyn Hasher) {
        self.hash(&mut HasherWrapper(state));
    }
}

/// Adapts a `&mut dyn Hasher` so it can be handed to [`Hash::hash`].
///
/// The wrapped hasher does all of the work; this type only exists to give the
/// borrow a concrete [`Hasher`] implementation.
struct HasherWrapper<'a>(&'a mut dyn Hasher);

impl Hasher for HasherWrapper<'_> {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

/// Compares boxed errors by concrete type and identity.
///
/// Two errors are equal only when they share the same concrete [`Any::type_id`]
/// *and* refer to the same allocation. Value based equality is not provided
/// because [`std::error::Error`] does not require [`PartialEq`].
impl PartialEq for Box<dyn AnyError> {
    fn eq(&self, other: &Self) -> bool {
        if self.as_any().type_id() != other.as_any().type_id() {
            return false;
        }
        ptr::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for Box<dyn AnyError> {}

impl Hash for Box<dyn AnyError> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_ref().hash_dyn(state);
    }
}

dyn_clone::clone_trait_object!(AnyError where Self: Send + Sync);

#[cfg(test)]
mod tests {
    use std::{
        fmt,
        hash::{Hash, Hasher},
    };

    use super::AnyError;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestError {
        code: u16,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "test error {}", self.code)
        }
    }

    impl std::error::Error for TestError {}

    impl Hash for TestError {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.code.hash(state);
        }
    }

    #[test]
    fn to_boxed_preserves_concrete_error_type() {
        let error = TestError { code: 42 };

        let boxed = error.to_boxed();
        let actual = boxed.as_any().downcast_ref::<TestError>();

        assert_eq!(actual, Some(&TestError { code: 42 }));
    }
}
