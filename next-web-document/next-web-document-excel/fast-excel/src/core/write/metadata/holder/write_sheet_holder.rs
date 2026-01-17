use std::{collections::HashMap, sync::Arc};

use crate::core::{
    enums::write_last_row_type::WriteLastRowType,
    poi::ss::usermodel::sheet::Sheet,
    write::{
        metadata::{
            holder::{
                base_write_holder::BaseWriteHolder, write_table_holder::WriteTableHolder,
                write_workbook_holder::WriteWorkbookHolder,
            },
            write_sheet::WriteSheet,
        },
        property::excel_write_head_property::ExcelWriteHeadProperty,
    },
};

/// Sheet holder for managing Excel sheet state during write operations.
///
/// This struct holds POI sheet instances, row tracking, table initialization state,
/// and other metadata required for writing to an Excel sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct WriteSheetHolder {
    /// Current write sheet configuration
    write_sheet: Arc<WriteSheet>,

    /// Current POI Sheet for writing.
    ///
    /// This may not contain data when reading template data in XLSX format.
    /// - XLS (03 format): `HSSFSheet`
    /// - XLSX (07 format): `SXSSFSheet`
    sheet: Arc<dyn Sheet>,

    /// Current POI Sheet for reading template data.
    ///
    /// Always use this method when reading template data.
    /// - XLS (03 format): `HSSFSheet`
    /// - XLSX (07 format): `XSSFSheet`
    cached_sheet: Arc<dyn Sheet>,

    /// Sheet number (0-based index)
    sheet_no: u32,

    /// Sheet name
    sheet_name: Option<String>,

    /// Parent workbook holder
    parent_write_workbook_holder: Arc<WriteWorkbookHolder>,

    /// Map of initialized tables by their table index
    has_been_initialized_table: HashMap<u32, Arc<WriteTableHolder>>,

    /// last column type
    write_last_row_type: WriteLastRowType,

    /// last row index
    last_row_index: u32,

    base: BaseWriteHolder,
}

impl WriteSheetHolder {
    /// Creates a new WriteSheetHolder from a WriteSheet configuration and parent workbook holder.
    ///
    /// # Arguments
    /// * `write_sheet` - Write sheet configuration
    /// * `write_workbook_holder` - Parent workbook holder
    pub fn new(
        write_sheet: Arc<WriteSheet>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
    ) -> Self {
        // Initialize handlers
        let handler = Self::init_handler(&write_sheet, &write_workbook_holder);

        // Determine sheet number and name
        let (sheet_no, sheet_name) = Self::determine_sheet_info(&write_sheet);

        // Get sheets from workbook holder
        let (sheet, cached_sheet) =
            write_workbook_holder.get_or_create_sheets(sheet_no, &sheet_name);

        // Determine initial write last row type
        let write_last_row_type = if write_workbook_holder
            .get_temp_template_input_stream()
            .is_some()
        {
            WriteLastRowType::TemplateEmpty
        } else {
            WriteLastRowType::CommonEmpty
        };

        Self {
            write_sheet: write_sheet.clone(),
            sheet: Arc::new(sheet),
            cached_sheet: Arc::new(cached_sheet),
            parent_write_workbook_holder: write_workbook_holder,
            has_been_initialized_table: HashMap::new(),
            sheet_no,
            sheet_name: sheet_name.filter(|s| !s.is_empty()).unwrap_or_default(),
            write_last_row_type,
            last_row_index: 0,

            base: Default::default(),
        }
    }

    /// Determines sheet number and name from WriteSheet configuration.
    fn determine_sheet_info(write_sheet: &WriteSheet) -> (u32, Option<String>) {
        match (write_sheet.sheet_no(), write_sheet.sheet_name()) {
            (Some(sheet_no), _) => (sheet_no, write_sheet.sheet_name()),
            (None, Some(sheet_name)) => (0, sheet_name),
            (None, _) => (0, None),
        }
    }

    /// Gets the next row index to start writing data.
    ///
    /// This method calculates the appropriate row index to start writing new data
    /// based on the current sheet state (template, empty, or already has data).
    ///
    /// # Returns
    /// * `u32` - The next row index to start writing data
    pub fn get_new_row_index_and_start_do_write(&mut self) -> u32 {
        // 'get_last_row_num' returns 0 if there's one or zero rows
        let new_row_index = match self.write_last_row_type {
            WriteLastRowType::TemplateEmpty => {
                // For template sheets, find the maximum last row from both sheet and cached sheet
                let sheet_last_row = self.sheet.get_last_row_num();
                let cached_last_row = self.cached_sheet.get_last_row_num();
                let max_last_row = sheet_last_row.max(cached_last_row);

                // If there's no data (first row is null), start at 0, otherwise increment
                if max_last_row != 0 || self.cached_sheet.get_row(0).is_some() {
                    max_last_row + 1
                } else {
                    max_last_row
                }
            }
            WriteLastRowType::HasData => {
                // For sheets that already have data, just increment the last row index
                let sheet_last_row = self.sheet.get_last_row_num();
                let cached_last_row = self.cached_sheet.get_last_row_num();
                sheet_last_row.max(cached_last_row) + 1
            }

            // Add other variants if needed
            _ => 0,
        };

        // Update state to indicate sheet now has data
        self.write_last_row_type = WriteLastRowType::HasData;

        new_row_index
    }

    /// Gets a mutable reference to the sheet.
    ///
    /// This is the main sheet used for writing operations.
    pub fn sheet(&self) -> &Sheet {
        &self.sheet
    }

    /// Gets the cached sheet for reading template data.
    pub fn cached_sheet(&self) -> &dyn Sheet {
        &self.cached_sheet
    }

    /// Gets the sheet number.
    pub fn sheet_no(&self) -> i32 {
        self.sheet_no
    }

    /// Gets the sheet name.
    pub fn sheet_name(&self) -> &str {
        &self.sheet_name
    }

    /// Gets the parent workbook holder.
    pub fn parent_write_workbook_holder(&self) -> &Arc<WriteWorkbookHolder> {
        &self.parent_write_workbook_holder
    }

    /// Gets the write sheet configuration.
    pub fn write_sheet(&self) -> &Arc<WriteSheet> {
        &self.write_sheet
    }

    /// Gets the map of initialized tables.
    pub fn has_been_initialized_table(&self) -> &HashMap<i32, Arc<WriteTableHolder>> {
        &self.has_been_initialized_table
    }

    pub fn get_excel_write_head_property(&self) -> &ExcelWriteHeadProperty {
        &self.excel_write_head_property
    }

    /// Gets a mutable reference to the map of initialized tables.
    pub fn has_been_initialized_table_mut(&mut self) -> &mut HashMap<i32, Arc<WriteTableHolder>> {
        &mut self.has_been_initialized_table
    }

    /// Gets the last row type.
    pub fn write_last_row_type(&self) -> WriteLastRowType {
        self.write_last_row_type
    }

    /// Sets the last row type.
    pub fn set_write_last_row_type(&mut self, write_last_row_type: WriteLastRowType) {
        self.write_last_row_type = write_last_row_type;
    }

    /// Gets the last row index.
    pub fn last_row_index(&self) -> i32 {
        self.last_row_index
    }

    /// Sets the last row index.
    pub fn set_last_row_index(&mut self, last_row_index: i32) {
        self.last_row_index = last_row_index;
    }

    /// Checks if this is a new sheet (no data written yet).
    pub fn is_new(&self) -> bool {
        matches!(
            self.write_last_row_type,
            WriteLastRowType::CommonEmpty | WriteLastRowType::TemplateEmpty
        )
    }

    /// Registers an initialized table.
    ///
    /// # Arguments
    /// * `table_index` - The index of the table
    /// * `table_holder` - The table holder instance
    pub fn register_initialized_table(
        &mut self,
        table_index: i32,
        table_holder: Arc<WriteTableHolder>,
    ) {
        self.has_been_initialized_table
            .insert(table_index, table_holder);
    }

    /// Checks if a table has been initialized.
    ///
    /// # Arguments
    /// * `table_index` - The table index to check
    ///
    /// # Returns
    /// * `bool` - `true` if the table has been initialized, `false` otherwise
    pub fn is_table_initialized(&self, table_index: i32) -> bool {
        self.has_been_initialized_table.contains_key(&table_index)
    }

    /// Gets an initialized table holder if it exists.
    ///
    /// # Arguments
    /// * `table_index` - The table index
    ///
    /// # Returns
    /// * `Option<&Arc<WriteTableHolder>>` - The table holder if it exists
    pub fn get_initialized_table(&self, table_index: i32) -> Option<&Arc<WriteTableHolder>> {
        self.has_been_initialized_table.get(&table_index)
    }

    /// Updates the sheet instance (e.g., when switching between HSSF and SXSSF).
    ///
    /// # Arguments
    /// * `new_sheet` - The new sheet instance
    pub fn update_sheet(&mut self, new_sheet: &dyn Sheet) {
        self.sheet = Arc::new(new_sheet);
    }

    /// Updates the cached sheet instance.
    ///
    /// # Arguments
    /// * `new_cached_sheet` - The new cached sheet instance
    pub fn update_cached_sheet(&mut self, new_cached_sheet: &dyn Sheet) {
        self.cached_sheet = Arc::new(new_cached_sheet);
    }

    /// Increments the last row index by one.
    pub fn increment_last_row_index(&mut self) {
        self.last_row_index += 1;
    }

    /// Resets the sheet holder to its initial state.
    ///
    /// This clears initialized tables and resets row tracking.
    pub fn reset(&mut self) {
        self.has_been_initialized_table.clear();
        self.last_row_index = 0;
        self.write_last_row_type = if self
            .parent_write_workbook_holder
            .temp_template_input_stream()
            .is_some()
        {
            WriteLastRowType::TemplateEmpty
        } else {
            WriteLastRowType::CommonEmpty
        };
    }
}

impl BaseWriteHolder for WriteSheetHolder {
    /// Returns the holder type (SHEET).
    fn holder_type(&self) -> HolderEnum {
        HolderEnum::Sheet
    }

    /// Gets the parent holder.
    fn parent_holder(&self) -> Option<&dyn AbstractWriteHolder> {
        Some(self.parent_write_workbook_holder.as_ref())
    }

    /// Gets the Excel write head property.
    fn excel_write_head_property(&self) -> &crate::metadata::ExcelWriteHeadProperty {
        self.write_sheet.excel_write_head_property()
    }
}

// Implement Default trait for convenience
impl Default for WriteSheetHolder {
    fn default() -> Self {
        Self {
            write_sheet: Arc::new(WriteSheet::default()),
            sheet: Arc::new(Sheet::default()),
            cached_sheet: Arc::new(Sheet::default()),
            sheet_no: 0,
            sheet_name: String::new(),
            parent_write_workbook_holder: Arc::new(WriteWorkbookHolder::default()),
            has_been_initialized_table: HashMap::new(),
            write_last_row_type: WriteLastRowType::CommonEmpty,
            last_row_index: 0,
        }
    }
}

// Add helper methods for sheet operations
impl WriteSheetHolder {
    /// Creates a row at the specified index.
    ///
    /// # Arguments
    /// * `row_index` - The row index (0-based)
    ///
    /// # Returns
    /// * `Result<poi::Row, poi::error::Error>` - The created row or an error
    pub fn create_row(&mut self, row_index: i32) -> Result<poi::Row, poi::error::Error> {
        let row = self.sheet_mut().create_row(row_index)?;
        self.last_row_index = self.last_row_index.max(row_index);
        Ok(row)
    }

    /// Gets a row at the specified index.
    ///
    /// # Arguments
    /// * `row_index` - The row index (0-based)
    ///
    /// # Returns
    /// * `Option<poi::Row>` - The row if it exists
    pub fn get_row(&self, row_index: i32) -> Option<poi::Row> {
        self.sheet.get_row(row_index)
    }

    /// Removes a row at the specified index.
    ///
    /// # Arguments
    /// * `row_index` - The row index (0-based)
    pub fn remove_row(&mut self, row_index: i32) -> Result<(), poi::error::Error> {
        self.sheet_mut().remove_row(row_index)?;
        if row_index == self.last_row_index {
            self.last_row_index = self.last_row_index.saturating_sub(1);
        }
        Ok(())
    }

    /// Gets the number of rows in the sheet.
    ///
    /// # Returns
    /// * `i32` - The number of rows
    pub fn get_physical_number_of_rows(&self) -> i32 {
        self.sheet.get_physical_number_of_rows()
    }
}
