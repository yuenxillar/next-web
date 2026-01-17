use crate::core::context::write_context::WriteContext;
use crate::core::error::excel_error::ExcelError;
use crate::core::metadata::head::Head;
use crate::core::poi::ss::usermodel::row::Row;
use crate::core::util::work_book_util::WorkBookUtil;
use crate::core::util::write_handler_utils::WriteHandlerUtils;
use crate::core::write::metadata::row_data::RowData;
use std::any::Any;
use std::collections::HashSet;

pub struct ExcelWriteAddExecutor<C> {
    write_context: C,
}

impl<C> ExcelWriteAddExecutor<C>
where
    C: WriteContext,
{
    pub fn new(write_context: C) -> Self {
        Self { write_context }
    }

    /// Adds a collection of data to the Excel sheet.
    ///
    /// # Arguments
    /// * `data` - The data to add to the Excel sheet. Can be empty.
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result.
    pub fn add(&self, data: &[RowData]) -> Result<(), ExcelError> {
        let data = if data.is_empty() {
            Vec::new()
        } else {
            data.to_vec()
        };

        let write_sheet_holder = self.write_context.write_sheet_holder();
        let mut new_row_index = write_sheet_holder.get_new_row_index_and_start_do_write();

        // Adjust row index if this is a new sheet without headers
        if write_sheet_holder.is_new()
            && !write_sheet_holder
                .get_excel_write_head_property()
                .has_head()
        {
            new_row_index += self
                .write_context
                .current_write_holder()
                .relative_head_row_index();
        }

        let mut relative_row_index = 0;
        for (index, one_row_data) in data.into_iter().enumerate() {
            let last_row_index = relative_row_index + new_row_index;
            self.add_one_row_of_data_to_excel(one_row_data, last_row_index, relative_row_index)?;

            relative_row_index += 1;
        }

        Ok(())
    }

    /// Adds a single row of data to the Excel sheet.
    ///
    /// # Arguments
    /// * `one_row_data` - The data for one row
    /// * `row_index` - The absolute row index in the sheet
    /// * `relative_row_index` - The relative row index in the current batch
    ///
    /// # Returns
    /// * `Result<()>` - Success or error result
    fn add_one_row_of_data_to_excel(
        &self,
        one_row_data: RowData,
        row_index: u32,
        relative_row_index: u32,
    ) -> Result<(), ExcelError> {
        // Create row write handler context
        let mut row_write_handler_context = WriteHandlerUtils::create_row_write_handler_context(
            &self.write_context,
            row_index,
            relative_row_index,
            false,
        );

        // Execute before row create handlers
        WriteHandlerUtils::before_row_create(&mut row_write_handler_context)?;

        // Create the row
        let sheet = self.write_context.write_sheet_holder().sheet();
        let row = WorkBookUtil::create_row(sheet, row_index);
        row_write_handler_context.set_row(row.clone());

        // Execute after row create handlers
        WriteHandlerUtils::after_row_create(&mut row_write_handler_context);

        // Handle different data types
        if one_row_data.is_collection() {
            self.add_basic_type_to_excel(one_row_data, row, row_index, relative_row_index)?;
        } else {
            self.add_object_to_excel(one_row_data, row, row_index, relative_row_index)?;
        }

        // Execute after row dispose handlers
        WriteHandlerUtils::after_row_dispose(&mut row_write_handler_context);

        Ok(())
    }

    /// Adds basic type data (collection or map) to Excel.
    fn add_basic_type_to_excel(
        &self,
        one_row_data: RowData,
        row: &dyn Row,
        row_index: u32,
        relative_row_index: u32,
    ) -> Result<(), ExcelError> {
        if one_row_data.is_empty() || one_row_data.is_object() {
            return Ok(());
        }

        let head_map = self
            .write_context
            .current_write_holder()
            .excel_write_head_property()
            .get_head_map();

        let mut data_index = 0;
        let mut max_cell_index = -1;

        // Write data according to header mapping
        for (column_index, head) in head_map.iter() {
            if data_index >= one_row_data.size() {
                return Ok(());
            }

            self.do_add_basic_type_to_excel(
                &one_row_data,
                Some(head),
                row,
                row_index,
                relative_row_index,
                data_index,
                column_index,
            )?;

            data_index += 1;
            max_cell_index = max_cell_index.max(column_index);
        }

        // If there's still data left, write it to subsequent cells
        if data_index >= one_row_data.size() {
            return Ok(());
        }

        // Fix for issue #1702: Write remaining data to next cells
        let mut next_cell_index = max_cell_index + 1;
        let remaining_size = one_row_data.size() - data_index;

        for i in 0..remaining_size {
            self.do_add_basic_type_to_excel(
                &one_row_data,
                None,
                row,
                row_index,
                relative_row_index,
                data_index,
                next_cell_index,
            )?;
            data_index += 1;
            next_cell_index += 1;
        }

        Ok(())
    }

    /// Performs the actual addition of basic type data to a cell.
    fn do_add_basic_type_to_excel(
        &self,
        one_row_data: &RowData,
        head: Option<&Head>,
        row: &dyn Row,
        row_index: i32,
        relative_row_index: i32,
        data_index: i32,
        column_index: i32,
    ) -> Result<(), ExcelError> {
        let field_name = head.map(|h| h.field_name().to_string());
        let head_clazz = self
            .write_context
            .current_write_holder()
            .excel_write_head_property()
            .head_class()
            .cloned();

        let excel_content_property = ClassUtils::declared_excel_content_property(
            None,
            head_clazz.as_deref(),
            field_name.as_deref(),
            self.write_context.current_write_holder(),
        );

        let mut cell_write_handler_context = WriteHandlerUtils::create_cell_write_handler_context(
            self.write_context,
            row.clone(),
            row_index,
            head,
            column_index,
            relative_row_index,
            false,
            excel_content_property,
        );

        // Execute before cell create handlers
        WriteHandlerUtils::before_cell_create(&mut cell_write_handler_context)?;

        // Create the cell
        let cell = WorkBookUtil::create_cell(row, column_index)?;
        cell_write_handler_context.set_cell(cell.clone());

        // Execute after cell create handlers
        WriteHandlerUtils::after_cell_create(&mut cell_write_handler_context)?;

        // Set original value and perform conversion
        let original_value = one_row_data.get(data_index as usize);
        cell_write_handler_context.set_original_value(original_value.clone());

        let original_field_class =
            FieldUtils::get_field_class(&cell_write_handler_context.original_value());
        cell_write_handler_context.set_original_field_class(original_field_class);

        // Convert and set the cell value
        self.converter_and_set(&mut cell_write_handler_context)?;

        // Execute after cell dispose handlers
        WriteHandlerUtils::after_cell_dispose(&mut cell_write_handler_context)?;

        Ok(())
    }

    /// Adds a Java-like object (struct) to Excel.
    fn add_object_to_excel<T>(
        &self,
        one_row_data: T,
        row: &dyn Row,
        row_index: i32,
        relative_row_index: i32,
    ) -> Result<(), ExcelError>
    where
        T: Any,
    {
        let current_write_holder = self.write_context.current_write_holder();

        // Create a bean map from the object
        let bean_map = match ClassUtils::create_bean_map(&one_row_data) {
            Some(map) => map,
            None => {
                // Fall back to reflection-based approach
                return self.add_object_via_reflection(
                    one_row_data,
                    row,
                    row_index,
                    relative_row_index,
                );
            }
        };

        let bean_key_set: HashSet<String> = bean_map.keys().cloned().collect();
        let mut bean_map_handled_set = HashSet::new();
        let mut max_cell_index = -1;

        // Handle class-based headers (type-casted)
        if HeadKind::Class == current_write_holder.excel_write_head_property().head_kind() {
            let head_map = current_write_holder.excel_write_head_property().head_map();

            for (&column_index, head) in head_map.iter() {
                let name = head.field_name().to_string();

                if !bean_key_set.contains(&name) {
                    continue;
                }

                let excel_content_property = ClassUtils::declared_excel_content_property(
                    Some(&bean_map),
                    current_write_holder
                        .excel_write_head_property()
                        .head_class()
                        .as_deref(),
                    Some(&name),
                    current_write_holder,
                );

                let mut cell_write_handler_context =
                    WriteHandlerUtils::create_cell_write_handler_context(
                        self.write_context,
                        row.clone(),
                        row_index,
                        Some(head),
                        column_index,
                        relative_row_index,
                        false,
                        excel_content_property,
                    );

                WriteHandlerUtils::before_cell_create(&mut cell_write_handler_context)?;

                let cell = WorkBookUtil::create_cell(row, column_index)?;
                cell_write_handler_context.set_cell(cell.clone());

                WriteHandlerUtils::after_cell_create(&mut cell_write_handler_context)?;

                if let Some(value) = bean_map.get(&name) {
                    cell_write_handler_context.set_original_value(value.clone());
                }

                let field_type = head.field().field_type();
                cell_write_handler_context.set_original_field_class(field_type);

                self.converter_and_set(&mut cell_write_handler_context)?;

                WriteHandlerUtils::after_cell_dispose(&mut cell_write_handler_context)?;

                bean_map_handled_set.insert(name);
                max_cell_index = max_cell_index.max(column_index);
            }
        }

        // If all bean map entries have been processed, return
        if bean_map_handled_set.len() == bean_map.len() {
            return Ok(());
        }

        // Process remaining fields
        let mut next_cell_index = max_cell_index + 1;

        // Get field cache for reflection-based processing
        let field_cache =
            ClassUtils::declared_fields(&one_row_data, self.write_context.current_write_holder());

        for (_, field_wrapper) in field_cache.sorted_field_map() {
            let field_name = field_wrapper.field_name();

            let is_useless_data =
                !bean_key_set.contains(field_name) || bean_map_handled_set.contains(field_name);

            if is_useless_data {
                continue;
            }

            let value = bean_map.get(field_name).cloned();

            let excel_content_property = ClassUtils::declared_excel_content_property(
                Some(&bean_map),
                current_write_holder
                    .excel_write_head_property()
                    .head_class()
                    .as_deref(),
                Some(field_name),
                current_write_holder,
            );

            let mut cell_write_handler_context =
                WriteHandlerUtils::create_cell_write_handler_context(
                    self.write_context,
                    row.clone(),
                    row_index,
                    None,
                    next_cell_index,
                    relative_row_index,
                    false,
                    excel_content_property,
                );

            WriteHandlerUtils::before_cell_create(&mut cell_write_handler_context)?;

            // Fix for issue #1870: Write data to next available cell
            let cell = WorkBookUtil::create_cell(row, next_cell_index)?;
            cell_write_handler_context.set_cell(cell.clone());

            WriteHandlerUtils::after_cell_create(&mut cell_write_handler_context)?;

            if let Some(val) = value {
                cell_write_handler_context.set_original_value(val);
            }

            let field_class = FieldUtils::get_field_class_for_map(&bean_map, field_name, &value);
            cell_write_handler_context.set_original_field_class(field_class);

            self.converter_and_set(&mut cell_write_handler_context)?;

            WriteHandlerUtils::after_cell_dispose(&mut cell_write_handler_context)?;

            next_cell_index += 1;
        }

        Ok(())
    }
}
