use std::sync::Arc;

/// Context for row write handlers during Excel writing operations.
///
/// This struct provides all necessary context information for handlers that
/// need to process rows during Excel writing, including:
/// - Write context for configuration and state
/// - Workbook, sheet, and table holders
/// - Row metadata (index, relative index, whether it's a header row)
#[derive(Debug, Clone)]
pub struct RowWriteHandlerContext {
    /// Write context containing configuration and state information
    write_context: Arc<WriteContext>,

    /// Workbook holder for workbook-level operations and metadata
    write_workbook_holder: Arc<WriteWorkbookHolder>,

    /// Sheet holder for sheet-level operations and metadata
    write_sheet_holder: Arc<WriteSheetHolder>,

    /// Table holder for table-level operations (nullable)
    /// This is `None` when not using table writes
    write_table_holder: Option<Arc<WriteTableHolder>>,

    /// Absolute row index in the sheet (0-based)
    row_index: i32,

    /// POI Row object for direct row operations
    row: Option<poi::Row>,

    /// Relative row index within the current batch of data (nullable)
    /// This is `None` in the case of fill data operations
    relative_row_index: Option<i32>,

    /// Whether this row is a header row (nullable)
    /// This is `None` in the case of fill data operations
    head: Option<bool>,
}

impl RowWriteHandlerContext {
    /// Creates a new RowWriteHandlerContext with the provided parameters.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `write_table_holder` - Optional table holder
    /// * `row_index` - Absolute row index in the sheet
    /// * `row` - Optional POI Row object
    /// * `relative_row_index` - Optional relative row index
    /// * `head` - Optional flag indicating if this is a header row
    pub fn new(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        write_table_holder: Option<Arc<WriteTableHolder>>,
        row_index: i32,
        row: Option<poi::Row>,
        relative_row_index: Option<i32>,
        head: Option<bool>,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder,
            row_index,
            row,
            relative_row_index,
            head,
        }
    }

    /// Creates a new RowWriteHandlerContext with default values for nullable fields.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `row_index` - Absolute row index in the sheet
    ///
    /// # Returns
    /// * `Self` - New context instance with default nullable values
    pub fn with_defaults(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        row_index: i32,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: None,
            row_index,
            row: None,
            relative_row_index: None,
            head: None,
        }
    }

    /// Creates a new RowWriteHandlerContext for a header row.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `row_index` - Absolute row index in the sheet
    /// * `row` - POI Row object for the header
    ///
    /// # Returns
    /// * `Self` - New context instance configured for a header row
    pub fn for_header(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        row_index: i32,
        row: poi::Row,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: None,
            row_index,
            row: Some(row),
            relative_row_index: Some(0),
            head: Some(true),
        }
    }

    /// Creates a new RowWriteHandlerContext for a data row.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `row_index` - Absolute row index in the sheet
    /// * `relative_row_index` - Relative row index within the data batch
    /// * `row` - POI Row object for the data
    ///
    /// # Returns
    /// * `Self` - New context instance configured for a data row
    pub fn for_data(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        row_index: i32,
        relative_row_index: i32,
        row: poi::Row,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: None,
            row_index,
            row: Some(row),
            relative_row_index: Some(relative_row_index),
            head: Some(false),
        }
    }

    /// Creates a new RowWriteHandlerContext for table operations.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `write_table_holder` - Table holder
    /// * `row_index` - Absolute row index in the sheet
    /// * `relative_row_index` - Relative row index within the table
    /// * `row` - POI Row object
    /// * `head` - Whether this is a header row
    ///
    /// # Returns
    /// * `Self` - New context instance configured for table operations
    pub fn for_table(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        write_table_holder: Arc<WriteTableHolder>,
        row_index: i32,
        relative_row_index: i32,
        row: poi::Row,
        head: bool,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: Some(write_table_holder),
            row_index,
            row: Some(row),
            relative_row_index: Some(relative_row_index),
            head: Some(head),
        }
    }

    /// Creates a new RowWriteHandlerContext for fill data operations.
    ///
    /// # Arguments
    /// * `write_context` - Write context for the operation
    /// * `write_workbook_holder` - Workbook holder
    /// * `write_sheet_holder` - Sheet holder
    /// * `row_index` - Absolute row index in the sheet
    ///
    /// # Returns
    /// * `Self` - New context instance configured for fill operations
    pub fn for_fill(
        write_context: Arc<WriteContext>,
        write_workbook_holder: Arc<WriteWorkbookHolder>,
        write_sheet_holder: Arc<WriteSheetHolder>,
        row_index: i32,
    ) -> Self {
        Self {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: None,
            row_index,
            row: None,
            relative_row_index: None,
            head: None,
        }
    }

    // Getters

    /// Gets the write context.
    pub fn write_context(&self) -> &Arc<WriteContext> {
        &self.write_context
    }

    /// Gets the workbook holder.
    pub fn write_workbook_holder(&self) -> &Arc<WriteWorkbookHolder> {
        &self.write_workbook_holder
    }

    /// Gets the sheet holder.
    pub fn write_sheet_holder(&self) -> &Arc<WriteSheetHolder> {
        &self.write_sheet_holder
    }

    /// Gets the table holder, if available.
    pub fn write_table_holder(&self) -> Option<&Arc<WriteTableHolder>> {
        self.write_table_holder.as_ref()
    }

    /// Gets the absolute row index.
    pub fn row_index(&self) -> i32 {
        self.row_index
    }

    /// Gets the POI Row object, if available.
    pub fn row(&self) -> Option<&poi::Row> {
        self.row.as_ref()
    }

    /// Gets a mutable reference to the POI Row object, if available.
    pub fn row_mut(&mut self) -> Option<&mut poi::Row> {
        self.row.as_mut()
    }

    /// Gets the relative row index, if available.
    pub fn relative_row_index(&self) -> Option<i32> {
        self.relative_row_index
    }

    /// Gets whether this is a header row, if specified.
    pub fn head(&self) -> Option<bool> {
        self.head
    }

    /// Checks if this context has a row object.
    pub fn has_row(&self) -> bool {
        self.row.is_some()
    }

    /// Checks if this context has a table holder.
    pub fn has_table(&self) -> bool {
        self.write_table_holder.is_some()
    }

    /// Checks if this is definitely a header row.
    ///
    /// # Returns
    /// * `bool` - `true` if head is specified and is `true`, `false` otherwise
    pub fn is_header_row(&self) -> bool {
        self.head.unwrap_or(false)
    }

    /// Checks if this is definitely a data row.
    ///
    /// # Returns
    /// * `bool` - `true` if head is specified and is `false`, `false` otherwise
    pub fn is_data_row(&self) -> bool {
        match self.head {
            Some(head) => !head,
            None => false,
        }
    }

    // Setters

    /// Sets the POI Row object.
    pub fn set_row(&mut self, row: poi::Row) {
        self.row = Some(row);
    }

    /// Sets the relative row index.
    pub fn set_relative_row_index(&mut self, relative_row_index: Option<i32>) {
        self.relative_row_index = relative_row_index;
    }

    /// Sets the header flag.
    pub fn set_head(&mut self, head: Option<bool>) {
        self.head = head;
    }

    /// Sets the table holder.
    pub fn set_write_table_holder(&mut self, write_table_holder: Option<Arc<WriteTableHolder>>) {
        self.write_table_holder = write_table_holder;
    }

    /// Updates the row index and resets the row object.
    ///
    /// This is useful when reusing a context for a different row.
    ///
    /// # Arguments
    /// * `new_row_index` - The new absolute row index
    pub fn update_row_index(&mut self, new_row_index: i32) {
        self.row_index = new_row_index;
        self.row = None; // Clear the row object since it's for a different row
    }

    /// Creates a deep copy of the context with a new row object.
    ///
    /// # Arguments
    /// * `new_row` - The new POI Row object
    ///
    /// # Returns
    /// * `Self` - A new context instance with the updated row
    pub fn with_row(&self, new_row: poi::Row) -> Self {
        Self {
            write_context: self.write_context.clone(),
            write_workbook_holder: self.write_workbook_holder.clone(),
            write_sheet_holder: self.write_sheet_holder.clone(),
            write_table_holder: self.write_table_holder.clone(),
            row_index: self.row_index,
            row: Some(new_row),
            relative_row_index: self.relative_row_index,
            head: self.head,
        }
    }

    /// Creates a deep copy of the context with updated metadata.
    ///
    /// # Arguments
    /// * `new_row_index` - The new absolute row index
    /// * `new_relative_row_index` - The new relative row index
    /// * `new_head` - The new header flag
    ///
    /// # Returns
    /// * `Self` - A new context instance with updated metadata
    pub fn with_metadata(
        &self,
        new_row_index: i32,
        new_relative_row_index: Option<i32>,
        new_head: Option<bool>,
    ) -> Self {
        Self {
            write_context: self.write_context.clone(),
            write_workbook_holder: self.write_workbook_holder.clone(),
            write_sheet_holder: self.write_sheet_holder.clone(),
            write_table_holder: self.write_table_holder.clone(),
            row_index: new_row_index,
            row: self.row.clone(),
            relative_row_index: new_relative_row_index,
            head: new_head,
        }
    }

    /// Creates a deep copy of the context with a table holder.
    ///
    /// # Arguments
    /// * `table_holder` - The table holder to add
    ///
    /// # Returns
    /// * `Self` - A new context instance with the table holder
    pub fn with_table_holder(&self, table_holder: Arc<WriteTableHolder>) -> Self {
        Self {
            write_context: self.write_context.clone(),
            write_workbook_holder: self.write_workbook_holder.clone(),
            write_sheet_holder: self.write_sheet_holder.clone(),
            write_table_holder: Some(table_holder),
            row_index: self.row_index,
            row: self.row.clone(),
            relative_row_index: self.relative_row_index,
            head: self.head,
        }
    }

    /// Converts the context to a builder for further modifications.
    pub fn to_builder(&self) -> RowWriteHandlerContextBuilder {
        RowWriteHandlerContextBuilder::from_context(self)
    }
}

/// Builder pattern for RowWriteHandlerContext.
///
/// This provides a fluent API for creating and modifying RowWriteHandlerContext instances.
pub struct RowWriteHandlerContextBuilder {
    write_context: Option<Arc<WriteContext>>,
    write_workbook_holder: Option<Arc<WriteWorkbookHolder>>,
    write_sheet_holder: Option<Arc<WriteSheetHolder>>,
    write_table_holder: Option<Arc<WriteTableHolder>>,
    row_index: i32,
    row: Option<poi::Row>,
    relative_row_index: Option<i32>,
    head: Option<bool>,
}

impl RowWriteHandlerContextBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            write_context: None,
            write_workbook_holder: None,
            write_sheet_holder: None,
            write_table_holder: None,
            row_index: 0,
            row: None,
            relative_row_index: None,
            head: None,
        }
    }

    /// Creates a builder from an existing context.
    pub fn from_context(context: &RowWriteHandlerContext) -> Self {
        Self {
            write_context: Some(context.write_context.clone()),
            write_workbook_holder: Some(context.write_workbook_holder.clone()),
            write_sheet_holder: Some(context.write_sheet_holder.clone()),
            write_table_holder: context.write_table_holder.clone(),
            row_index: context.row_index,
            row: context.row.clone(),
            relative_row_index: context.relative_row_index,
            head: context.head,
        }
    }

    /// Sets the write context.
    pub fn write_context(mut self, write_context: Arc<WriteContext>) -> Self {
        self.write_context = Some(write_context);
        self
    }

    /// Sets the workbook holder.
    pub fn write_workbook_holder(mut self, holder: Arc<WriteWorkbookHolder>) -> Self {
        self.write_workbook_holder = Some(holder);
        self
    }

    /// Sets the sheet holder.
    pub fn write_sheet_holder(mut self, holder: Arc<WriteSheetHolder>) -> Self {
        self.write_sheet_holder = Some(holder);
        self
    }

    /// Sets the table holder.
    pub fn write_table_holder(mut self, holder: Option<Arc<WriteTableHolder>>) -> Self {
        self.write_table_holder = holder;
        self
    }

    /// Sets the absolute row index.
    pub fn row_index(mut self, row_index: i32) -> Self {
        self.row_index = row_index;
        self
    }

    /// Sets the POI Row object.
    pub fn row(mut self, row: Option<poi::Row>) -> Self {
        self.row = row;
        self
    }

    /// Sets the relative row index.
    pub fn relative_row_index(mut self, relative_row_index: Option<i32>) -> Self {
        self.relative_row_index = relative_row_index;
        self
    }

    /// Sets the header flag.
    pub fn head(mut self, head: Option<bool>) -> Self {
        self.head = head;
        self
    }

    /// Builds the RowWriteHandlerContext.
    ///
    /// # Returns
    /// * `Result<RowWriteHandlerContext, BuildError>` - The built context or an error
    pub fn build(self) -> Result<RowWriteHandlerContext, BuildError> {
        let write_context = self
            .write_context
            .ok_or(BuildError::MissingField("write_context"))?;
        let write_workbook_holder = self
            .write_workbook_holder
            .ok_or(BuildError::MissingField("write_workbook_holder"))?;
        let write_sheet_holder = self
            .write_sheet_holder
            .ok_or(BuildError::MissingField("write_sheet_holder"))?;

        Ok(RowWriteHandlerContext {
            write_context,
            write_workbook_holder,
            write_sheet_holder,
            write_table_holder: self.write_table_holder,
            row_index: self.row_index,
            row: self.row,
            relative_row_index: self.relative_row_index,
            head: self.head,
        })
    }
}

impl Default for RowWriteHandlerContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}
