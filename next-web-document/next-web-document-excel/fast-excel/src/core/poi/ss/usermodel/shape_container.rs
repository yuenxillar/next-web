use crate::core::poi::ss::usermodel::shape::Shape;

/// A common interface for shape groups.
///
pub trait ShapeContainer<T: Shape>: std::iter::Iterator<Item = T> {
    // Note: In Rust, the `IntoIterator` trait serves the same purpose as Java's `Iterable<T>`.
    // The trait can remain empty if there are no additional methods.
}
