#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadDefaultReturn {
    String,
    ActualData,
    ReadCellData,
}
