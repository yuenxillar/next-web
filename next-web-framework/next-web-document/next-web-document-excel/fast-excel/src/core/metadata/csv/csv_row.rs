use std::cell::RefCell;
use std::rc::Rc;

use crate::core::enums::cell_data::CellType;
use crate::core::metadata::csv::csv_cell_style::CsvCellStyle;
use crate::core::metadata::csv::csv_sheet::CsvSheet;
use crate::core::metadata::csv::csv_workbook::CsvWorkbook;
use crate::core::usermodel::cell::Cell;
use crate::core::usermodel::cell_style::CellStyle;
use crate::core::usermodel::row::{MissingCellPolicy, Row};
use crate::core::usermodel::sheet::Sheet;

/// CSV row implementation
pub struct CsvRow {
    /// Cell list
    cell_list: Vec<Rc<RefCell<CsvCell>>>,

    /// Workbook reference
    csv_workbook: Rc<RefCell<CsvWorkbook>>,

    /// Sheet reference
    csv_sheet: Rc<RefCell<CsvSheet>>,

    /// Row index (0-based)
    row_index: i32,

    /// Style
    cell_style: Option<Rc<RefCell<CsvCellStyle>>>,
}

impl CsvRow {
    /// Creates a new CSV row
    pub fn new(
        csv_workbook: Rc<RefCell<CsvWorkbook>>,
        csv_sheet: Rc<RefCell<CsvSheet>>,
        row_index: i32,
    ) -> Self {
        CsvRow {
            cell_list: Vec::new(),
            csv_workbook,
            csv_sheet,
            row_index,
            cell_style: None,
        }
    }
}

impl Row for CsvRow {
    fn create_cell(&mut self, column: i32) -> &dyn Cell {
        let cell = Rc::new(RefCell::new(CsvCell::new(
            Rc::clone(&self.csv_workbook),
            Rc::clone(&self.csv_sheet),
            self.row_index,
            column,
            None,
        )));

        // Ensure we have enough cells in the list
        let required_len = column as usize + 1;
        if self.cell_list.len() < required_len {
            self.cell_list
                .resize(required_len, Rc::new(RefCell::new(CsvCell::empty())));
        }

        self.cell_list[column as usize] = Rc::clone(&cell);

        // cell

        todo!()
    }

    fn create_cell_with_type(&mut self, column: i32, cell_type: CellType) -> &dyn Cell {
        let cell = Rc::new(RefCell::new(CsvCell::new(
            Rc::clone(&self.csv_workbook),
            Rc::clone(&self.csv_sheet),
            self.row_index,
            column,
            Some(cell_type),
        )));

        // Ensure we have enough cells in the list
        let required_len = column as usize + 1;
        if self.cell_list.len() < required_len {
            self.cell_list
                .resize(required_len, Rc::new(RefCell::new(CsvCell::empty())));
        }

        self.cell_list[column as usize] = Rc::clone(&cell);

        // cell
        todo!()
    }

    fn remove_cell(&mut self, cell: &dyn Cell) {
        // Find and remove the cell from the list
        // self.cell_list.retain(|c| {
        //     let c_ref = c.borrow();
        //     !std::ptr::eq(&*c_ref as *const _, cell as *const _)
        // });
    }

    fn set_row_num(&mut self, row_num: i32) {
        self.row_index = row_num;

        // Update row index for all cells
        for cell in &self.cell_list {
            let mut cell_mut = cell.borrow_mut();
            if let Some(csv_cell) = cell_mut.as_any().downcast_mut::<CsvCell>() {
                csv_cell.set_row_index(row_num);
            }
        }
    }

    fn get_row_num(&self) -> i32 {
        self.row_index
    }

    fn get_cell(&self, column_index: i32) -> Option<&dyn Cell> {
        if column_index < 0 || column_index as usize >= self.cell_list.len() {
            return None;
        }

        let cell = &self.cell_list[column_index as usize];
        if cell.borrow().is_empty() {
            return None;
        }

        // Some(Rc::clone(cell))
        todo!()
    }

    fn get_cell_with_policy(
        &self,
        column_index: i32,
        _policy: MissingCellPolicy,
    ) -> Option<&dyn Cell> {
        // CSV implementation ignores the policy and always returns the same
        // self.get_cell(column_index)
        //
        todo!()
    }

    fn get_first_cell_num(&self) -> u16 {
        if self.cell_list.is_empty() {
            return u16::MAX; // Equivalent to -1 in signed context
        }

        // Find first non-empty cell
        for (i, cell) in self.cell_list.iter().enumerate() {
            if !cell.borrow().is_empty() {
                return i as u16;
            }
        }

        u16::MAX
    }

    fn get_last_cell_num(&self) -> u16 {
        if self.cell_list.is_empty() {
            return u16::MAX; // Equivalent to -1 in signed context
        }

        // Find last non-empty cell
        for (i, cell) in self.cell_list.iter().enumerate().rev() {
            if !cell.borrow().is_empty() {
                return i as u16;
            }
        }

        u16::MAX
    }

    fn get_physical_number_of_cells(&self) -> i32 {
        self.cell_list
            .iter()
            .filter(|cell| !cell.borrow().is_empty())
            .count() as i32
    }

    fn set_height(&mut self, _height: u16) {
        // CSV doesn't support row height
    }

    fn set_zero_height(&mut self, _zero_height: bool) {
        // CSV doesn't support row height
    }

    fn get_zero_height(&self) -> bool {
        false
    }

    fn set_height_in_points(&mut self, _height: f32) {
        // CSV doesn't support row height
    }

    fn get_height(&self) -> u16 {
        0
    }

    fn get_height_in_points(&self) -> f32 {
        0.0
    }

    fn is_formatted(&self) -> bool {
        self.cell_style.is_some()
    }

    fn get_row_style(&self) -> Option<&dyn CellStyle> {
        // self.cell_style
        //     .as_ref()
        //     .map(|style| Rc::clone(style) as Rc<RefCell<dyn CellStyle>>)
        todo!()
    }

    fn set_row_style(&mut self, style: &dyn CellStyle) {
        // We need to store a reference to the style
        // In a real implementation, you might want to clone or reference count it properly
        self.cell_style = None; // Placeholder - actual implementation depends on CellStyle type
    }

    fn cell_iterator(&self) -> &dyn Iterator<Item = &dyn Cell> {
        // Box::new(
        //     self.cell_list
        //         .iter()
        //         .filter(|cell| !cell.borrow().is_empty())
        //         .map(|cell| Rc::clone(cell) as Rc<RefCell<dyn Cell>>),
        // )

        todo!()
    }

    fn get_sheet(&self) -> &dyn Sheet {
        todo!()
    }

    fn get_outline_level(&self) -> i32 {
        0 // CSV doesn't support outline levels
    }

    fn shift_cells_right(&mut self, _start_column: u32, _end_column: u32, _n: u32) {
        // CSV doesn't support cell shifting
    }

    fn shift_cells_left(&mut self, _start_column: u32, _end_column: u32, _n: u32) {
        // CSV doesn't support cell shifting
    }
}

// Additional helper methods for CsvRow
impl CsvRow {
    /// Gets all cells as a vector
    pub fn get_cells(&self) -> &Vec<Rc<RefCell<CsvCell>>> {
        &self.cell_list
    }

    /// Gets mutable access to cells
    pub fn get_cells_mut(&mut self) -> &mut Vec<Rc<RefCell<CsvCell>>> {
        &mut self.cell_list
    }

    /// Gets the cell at the specified index, creating it if it doesn't exist
    pub fn get_or_create_cell(&mut self, column: i32) -> Rc<RefCell<CsvCell>> {
        let required_len = column as usize + 1;
        if self.cell_list.len() < required_len {
            self.cell_list
                .resize(required_len, Rc::new(RefCell::new(CsvCell::empty())));
        }

        let cell = &self.cell_list[column as usize];
        if cell.borrow().is_empty() {
            // Create a new cell
            let new_cell = Rc::new(RefCell::new(CsvCell::new(
                Rc::clone(&self.csv_workbook),
                Rc::clone(&self.csv_sheet),
                self.row_index,
                column,
                None,
            )));
            self.cell_list[column as usize] = Rc::clone(&new_cell);
            new_cell
        } else {
            Rc::clone(cell)
        }
    }

    /// Clears all cells in the row
    pub fn clear_cells(&mut self) {
        self.cell_list.clear();
    }

    /// Returns the number of cells (including empty ones)
    pub fn len(&self) -> usize {
        self.cell_list.len()
    }

    /// Checks if the row has no cells
    pub fn is_empty(&self) -> bool {
        self.cell_list.iter().all(|cell| cell.borrow().is_empty())
    }
}

// Supporting structs and traits (simplified)
pub struct CsvCell {
    // Implementation details
    row_index: i32,
    column_index: i32,
    cell_type: Option<CellType>,
    value: Option<String>,
}

impl CsvCell {
    pub fn new(
        _workbook: Rc<RefCell<CsvWorkbook>>,
        _sheet: Rc<RefCell<CsvSheet>>,
        row_index: i32,
        column_index: i32,
        cell_type: Option<CellType>,
    ) -> Self {
        CsvCell {
            row_index,
            column_index,
            cell_type,
            value: None,
        }
    }

    pub fn empty() -> Self {
        CsvCell {
            row_index: -1,
            column_index: -1,
            cell_type: None,
            value: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.row_index == -1 && self.column_index == -1
    }

    pub fn set_row_index(&mut self, row_index: i32) {
        self.row_index = row_index;
    }

    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
