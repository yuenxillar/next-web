#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum CellExtraType {
    Comment,
    Hyperlink,
    Merge,
}
