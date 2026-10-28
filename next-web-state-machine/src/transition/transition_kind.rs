/// Defines enumeration of a Transition kind. This is uses within a transition to indicate whether its type is external,
///  internal or local.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TransitionKind {
    /// Indicates an external transition kind.
    External,

    /// Indicates an internal transition kind.
    Internal,

    /// Indicates a local transition kind.
    Local,

    /// Indicates an initial transition kind.
    Initial,
}
