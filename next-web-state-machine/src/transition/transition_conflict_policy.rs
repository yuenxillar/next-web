/// The policy which decides which transition wins when several match.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TransitionConflictPolicy {
    /// Policy choosing transition from a child.
    Child,

    /// Policy choosing from a parent.
    Parent,
}
