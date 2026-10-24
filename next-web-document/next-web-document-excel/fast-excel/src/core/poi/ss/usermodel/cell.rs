use std::fmt::Debug;

use crate::core::poi::ss::usermodel::cell_style::CellStyle;

pub trait Cell: Debug {
    /// Row index
    fn get_row_index(&self) -> Option<u32>;

    /// Column index
    fn get_column_index(&self) -> Option<u32>;

    fn get_cell_style(&self) -> Option<&dyn CellStyle>;
}
