pub trait Cell {
    /// Row index
    fn get_row_index(&self) -> Option<u32>;

    /// Column index
    fn get_column_index(&self) -> Option<u32>;
}
