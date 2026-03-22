#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TransitionConflictPolicy {
    /// Policy choosing transition from a child.
    Child,

    /// Policy choosing from a parent.
    Parent,
}
