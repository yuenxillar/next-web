/// A type that exposes a stable, human-readable name.
///
/// The name is intended for logging, metrics, diagnostics, and error
/// messages. It is not required to be unique; if uniqueness within a
/// collection is needed, that must be enforced by the caller.
pub trait Named {
    /// Returns the name of this value.
    ///
    /// Implementations should return a stable string: the same value must
    /// yield the same name across calls. The returned slice borrows from
    /// `self`, so it lives only as long as this value.
    fn name(&self) -> &str;
}
