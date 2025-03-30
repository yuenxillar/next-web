/// Represents the color of the function, i.e., async or sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    /// async function
    Async,
    /// sync function
    Sync,
}