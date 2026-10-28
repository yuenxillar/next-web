use crate::core::metadata::csv::csv_row::CsvRow;

#[derive(Debug, Clone)]
pub struct CsvSheet {
    row_cache_count: Option<u32>,
    last_row_index: Option<u32>,
    row_cache: Option<Vec<CsvRow>>,
}
