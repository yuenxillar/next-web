use crate::core::{
    enums::cell_data::CellType,
    poi::ss::usermodel::{cell::Cell, cell_style::CellStyle, sheet::Sheet},
};

/// Represents a row in a spreadsheet with cell operations and formatting.
pub trait Row {
    // Cell creation
    fn create_cell(&mut self, column_index: i32) -> &dyn Cell;
    fn create_cell_with_type(&mut self, column_index: i32, cell_type: CellType) -> &dyn Cell;

    // Cell removal
    fn remove_cell(&mut self, cell: &dyn Cell);

    // Row number operations
    fn set_row_num(&mut self, row_num: i32);
    fn get_row_num(&self) -> i32;

    // Cell access
    fn get_cell(&self, column_index: i32) -> Option<&dyn Cell>;
    fn get_cell_with_policy(
        &self,
        column_index: i32,
        policy: MissingCellPolicy,
    ) -> Option<&dyn Cell>;

    // Cell range information
    fn get_first_cell_num(&self) -> u16;
    fn get_last_cell_num(&self) -> u16;
    fn get_physical_number_of_cells(&self) -> i32;

    // Height operations
    fn set_height(&mut self, height: u16);
    fn set_zero_height(&mut self, zero_height: bool);
    fn get_zero_height(&self) -> bool;
    fn set_height_in_points(&mut self, height: f32);
    fn get_height(&self) -> u16;
    fn get_height_in_points(&self) -> f32;

    // Row formatting
    fn is_formatted(&self) -> bool;
    fn get_row_style(&self) -> Option<&dyn CellStyle>;
    fn set_row_style(&mut self, style: &dyn CellStyle);

    // Cell iteration
    fn cell_iterator(&self) -> &dyn Iterator<Item = &dyn Cell>;

    // Sheet reference
    // fn get_sheet(&self) -> &dyn Sheet;

    // Outline level for grouping
    fn get_outline_level(&self) -> i32;

    // Cell shifting
    fn shift_cells_right(&mut self, start_column: u32, end_column: u32, n: u32);
    fn shift_cells_left(&mut self, start_column: u32, end_column: u32, n: u32);
}

/// Policy for handling missing cells when retrieving cell values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingCellPolicy {
    /// Return null for missing cells, including blank cells
    ReturnNullAndBlank,
    /// Return null for missing cells, treat blank cells as null
    ReturnBlankAsNull,
    /// Create blank cells for null entries
    CreateNullAsBlank,
}
