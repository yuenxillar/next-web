/// Defines enumeration of a PseudoState kind. This is used within a transitive states indicating its kind.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum PseudoStateKind {
    /// Indicates an initial kind.
    Initial,

    /// End or terminate kind
    End,

    /// Choice kind
    Choice,

    /// Junction kind
    Junction,

    /// History deep kind
    HistoryDeep,

    /// History shallow kind
    HistoryShallow,

    /// Fork kind
    Fork,

    /// Join kind
    Join,

    /// Entrypoint kind
    Entry,

    /// Exitpoint kind
    Exit,
}
