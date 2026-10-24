#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolderType {
    Workbook,
    Sheet,
    Table,
    Row,
}
