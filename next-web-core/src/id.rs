/// A type that exposes a stable, unique identifier.
///
/// The identifier is intended to distinguish this value from all other
/// live values of the same kind — for example, to key a registry, route a
/// request, or correlate log lines. Callers that rely on uniqueness must
/// ensure it themselves; this trait does not enforce it.
///
/// Unlike a human-readable name (see [`Named`]), an `id` should be treated
/// as opaque: it need not be meaningful to humans and must not be parsed
/// to recover state.
pub trait Id {
    /// Returns the identifier of this value.
    ///
    /// Implementations should return a stable string: the same value must
    /// yield the same id across calls, and two distinct live values of the
    /// same kind should not share an id. The returned slice borrows from
    /// `self`, so it lives only as long as this value.
    fn id(&self) -> &str;
}
