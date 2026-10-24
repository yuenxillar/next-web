pub struct BaseWriteHolder {}

// //! Abstract write holder for managing write configurations and handlers.
// //!
// //! This trait and its implementations manage the configuration for writing Excel files,
// //! including headers, converters, handlers, and column filtering.

// use std::any::TypeId;
// use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
// use std::sync::Arc;

// use crate::constant::OrderConstant;
// use crate::converters::{Converter, ConverterKeyBuild, DefaultConverterLoader};
// use crate::enums::HeadKind;
// use crate::event::NotRepeatExecutor;
// use crate::metadata::{
//     AbstractHolder, ExcelContentProperty, Head, LoopMergeProperty, OnceAbsoluteMergeProperty,
//     RowHeightProperty,
// };
// use crate::write::handler::DefaultWriteHandlerLoader;
// use crate::write::handler::chain::{
//     CellHandlerExecutionChain, RowHandlerExecutionChain, SheetHandlerExecutionChain,
//     WorkbookHandlerExecutionChain,
// };
// use crate::write::handler::context::CellWriteHandlerContext;
// use crate::write::handler::{
//     CellWriteHandler, RowWriteHandler, SheetWriteHandler, WorkbookWriteHandler, WriteHandler,
// };
// use crate::write::merge::{LoopMergeStrategy, OnceAbsoluteMergeStrategy};
// use crate::write::metadata::{WriteBasicParameter, WriteHolder};
// use crate::write::property::ExcelWriteHeadProperty;
// use crate::write::style::{
//     AbstractHeadColumnWidthStyleStrategy, AbstractVerticalCellStyleStrategy,
//     SimpleRowHeightStyleStrategy, WriteCellStyle,
// };

// use anyhow::{Context, Result};

// /// Abstract write holder base trait for all write holders.
// ///
// /// This trait defines the common interface for workbook, sheet, and table holders,
// /// managing configurations, converters, and handlers for Excel writing operations.
// pub trait AbstractWriteHolder: AbstractHolder + WriteHolder + Send + Sync {
//     /// Gets whether headers are needed.
//     fn need_head(&self) -> bool;

//     /// Sets whether headers are needed.
//     fn set_need_head(&mut self, need_head: bool);

//     /// Gets the relative header row index.
//     fn relative_head_row_index(&self) -> i32;

//     /// Sets the relative header row index.
//     fn set_relative_head_row_index(&mut self, index: i32);

//     /// Gets the Excel write head property.
//     fn excel_write_head_property(&self) -> &ExcelWriteHeadProperty;

//     /// Gets a mutable reference to the Excel write head property.
//     fn excel_write_head_property_mut(&mut self) -> &mut ExcelWriteHeadProperty;

//     /// Gets whether to use default style.
//     fn use_default_style(&self) -> bool;

//     /// Sets whether to use default style.
//     fn set_use_default_style(&mut self, use_default_style: bool);

//     /// Gets whether to automatically merge headers.
//     fn automatic_merge_head(&self) -> bool;

//     /// Sets whether to automatically merge headers.
//     fn set_automatic_merge_head(&mut self, automatic_merge_head: bool);

//     /// Gets the excluded column indexes.
//     fn exclude_column_indexes(&self) -> Option<&Vec<i32>>;

//     /// Gets the excluded column field names.
//     fn exclude_column_field_names(&self) -> Option<&Vec<String>>;

//     /// Gets the included column indexes.
//     fn include_column_indexes(&self) -> Option<&Vec<i32>>;

//     /// Gets the included column field names.
//     fn include_column_field_names(&self) -> Option<&Vec<String>>;

//     /// Gets whether to order by included columns.
//     fn order_by_include_column(&self) -> bool;

//     /// Sets whether to order by included columns.
//     fn set_order_by_include_column(&mut self, order_by_include_column: bool);

//     /// Gets the list of write handlers.
//     fn write_handler_list(&self) -> &Vec<Arc<dyn WriteHandler>>;

//     /// Gets a mutable reference to the list of write handlers.
//     fn write_handler_list_mut(&mut self) -> &mut Vec<Arc<dyn WriteHandler>>;

//     /// Gets the owned workbook handler execution chain.
//     fn own_workbook_handler_execution_chain(&self) -> Option<&WorkbookHandlerExecutionChain>;

//     /// Gets a mutable reference to the owned workbook handler execution chain.
//     fn own_workbook_handler_execution_chain_mut(
//         &mut self,
//     ) -> Option<&mut WorkbookHandlerExecutionChain>;

//     /// Gets the owned sheet handler execution chain.
//     fn own_sheet_handler_execution_chain(&self) -> Option<&SheetHandlerExecutionChain>;

//     /// Gets a mutable reference to the owned sheet handler execution chain.
//     fn own_sheet_handler_execution_chain_mut(&mut self) -> Option<&mut SheetHandlerExecutionChain>;

//     /// Gets the workbook handler execution chain.
//     fn workbook_handler_execution_chain(&self) -> Option<&WorkbookHandlerExecutionChain>;

//     /// Gets a mutable reference to the workbook handler execution chain.
//     fn workbook_handler_execution_chain_mut(
//         &mut self,
//     ) -> Option<&mut WorkbookHandlerExecutionChain>;

//     /// Gets the sheet handler execution chain.
//     fn sheet_handler_execution_chain(&self) -> Option<&SheetHandlerExecutionChain>;

//     /// Gets a mutable reference to the sheet handler execution chain.
//     fn sheet_handler_execution_chain_mut(&mut self) -> Option<&mut SheetHandlerExecutionChain>;

//     /// Gets the row handler execution chain.
//     fn row_handler_execution_chain(&self) -> Option<&RowHandlerExecutionChain>;

//     /// Gets a mutable reference to the row handler execution chain.
//     fn row_handler_execution_chain_mut(&mut self) -> Option<&mut RowHandlerExecutionChain>;

//     /// Gets the cell handler execution chain.
//     fn cell_handler_execution_chain(&self) -> Option<&CellHandlerExecutionChain>;

//     /// Gets a mutable reference to the cell handler execution chain.
//     fn cell_handler_execution_chain_mut(&mut self) -> Option<&mut CellHandlerExecutionChain>;

//     /// Initializes the write holder from a write basic parameter and parent holder.
//     ///
//     /// # Arguments
//     /// * `write_basic_parameter` - Write configuration parameters
//     /// * `parent_abstract_write_holder` - Parent write holder for inheritance
//     fn init(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) -> Result<()>;

//     /// Initializes handlers for the write holder.
//     ///
//     /// # Arguments
//     /// * `write_basic_parameter` - Write configuration parameters
//     /// * `parent_abstract_write_holder` - Parent write holder for inheritance
//     fn init_handler(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) -> Result<()>;

//     /// Initializes annotation-based configuration.
//     ///
//     /// # Arguments
//     /// * `handler_list` - List to add handlers to
//     /// * `write_basic_parameter` - Write configuration parameters
//     fn init_annotation_config(
//         &mut self,
//         handler_list: &mut Vec<Arc<dyn WriteHandler>>,
//         write_basic_parameter: &WriteBasicParameter,
//     ) -> Result<()>;

//     /// Checks if a field or column should be ignored based on include/exclude lists.
//     ///
//     /// # Arguments
//     /// * `field_name` - Optional field name to check
//     /// * `column_index` - Optional column index to check
//     ///
//     /// # Returns
//     /// * `bool` - `true` if the field/column should be ignored, `false` otherwise
//     fn ignore(&self, field_name: Option<&str>, column_index: Option<i32>) -> bool;
// }

// /// Default implementation for AbstractWriteHolder.
// #[derive(Debug)]
// pub struct DefaultWriteHolder {
//     // Configuration
//     need_head: bool,
//     relative_head_row_index: i32,
//     excel_write_head_property: ExcelWriteHeadProperty,
//     use_default_style: bool,
//     automatic_merge_head: bool,
//     exclude_column_indexes: Option<Vec<i32>>,
//     exclude_column_field_names: Option<Vec<String>>,
//     include_column_indexes: Option<Vec<i32>>,
//     include_column_field_names: Option<Vec<String>>,
//     order_by_include_column: bool,

//     // Handlers
//     write_handler_list: Vec<Arc<dyn WriteHandler>>,
//     own_workbook_handler_execution_chain: Option<WorkbookHandlerExecutionChain>,
//     own_sheet_handler_execution_chain: Option<SheetHandlerExecutionChain>,
//     workbook_handler_execution_chain: Option<WorkbookHandlerExecutionChain>,
//     sheet_handler_execution_chain: Option<SheetHandlerExecutionChain>,
//     row_handler_execution_chain: Option<RowHandlerExecutionChain>,
//     cell_handler_execution_chain: Option<CellHandlerExecutionChain>,

//     // Converters
//     converter_map: HashMap<TypeId, Arc<dyn Converter>>,

//     // Parent reference (weak reference to avoid cycles)
//     parent_holder: Option<Arc<dyn AbstractWriteHolder>>,
// }

// impl DefaultWriteHolder {
//     /// Creates a new DefaultWriteHolder.
//     pub fn new() -> Self {
//         Self {
//             need_head: true,
//             relative_head_row_index: 0,
//             excel_write_head_property: ExcelWriteHeadProperty::default(),
//             use_default_style: true,
//             automatic_merge_head: true,
//             exclude_column_indexes: None,
//             exclude_column_field_names: None,
//             include_column_indexes: None,
//             include_column_field_names: None,
//             order_by_include_column: false,
//             write_handler_list: Vec::new(),
//             own_workbook_handler_execution_chain: None,
//             own_sheet_handler_execution_chain: None,
//             workbook_handler_execution_chain: None,
//             sheet_handler_execution_chain: None,
//             row_handler_execution_chain: None,
//             cell_handler_execution_chain: None,
//             converter_map: HashMap::new(),
//             parent_holder: None,
//         }
//     }

//     /// Builds a handler chain from a write handler.
//     ///
//     /// # Arguments
//     /// * `write_handler` - The write handler to add to the chain
//     /// * `run_own` - Whether this is for the own (annotation-based) chain
//     fn build_chain(&mut self, write_handler: Arc<dyn WriteHandler>, run_own: bool) -> Result<()> {
//         // Try to downcast to specific handler types
//         if let Some(cell_handler) = write_handler.clone().downcast::<dyn CellWriteHandler>() {
//             if !run_own {
//                 if self.cell_handler_execution_chain.is_none() {
//                     self.cell_handler_execution_chain =
//                         Some(CellHandlerExecutionChain::new(cell_handler));
//                 } else {
//                     self.cell_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(cell_handler);
//                 }
//             }
//         }

//         if let Some(row_handler) = write_handler.clone().downcast::<dyn RowWriteHandler>() {
//             if !run_own {
//                 if self.row_handler_execution_chain.is_none() {
//                     self.row_handler_execution_chain =
//                         Some(RowHandlerExecutionChain::new(row_handler));
//                 } else {
//                     self.row_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(row_handler);
//                 }
//             }
//         }

//         if let Some(sheet_handler) = write_handler.clone().downcast::<dyn SheetWriteHandler>() {
//             if !run_own {
//                 if self.sheet_handler_execution_chain.is_none() {
//                     self.sheet_handler_execution_chain =
//                         Some(SheetHandlerExecutionChain::new(sheet_handler));
//                 } else {
//                     self.sheet_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(sheet_handler);
//                 }
//             } else {
//                 if self.own_sheet_handler_execution_chain.is_none() {
//                     self.own_sheet_handler_execution_chain =
//                         Some(SheetHandlerExecutionChain::new(sheet_handler));
//                 } else {
//                     self.own_sheet_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(sheet_handler);
//                 }
//             }
//         }

//         if let Some(workbook_handler) = write_handler.clone().downcast::<dyn WorkbookWriteHandler>()
//         {
//             if !run_own {
//                 if self.workbook_handler_execution_chain.is_none() {
//                     self.workbook_handler_execution_chain =
//                         Some(WorkbookHandlerExecutionChain::new(workbook_handler));
//                 } else {
//                     self.workbook_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(workbook_handler);
//                 }
//             } else {
//                 if self.own_workbook_handler_execution_chain.is_none() {
//                     self.own_workbook_handler_execution_chain =
//                         Some(WorkbookHandlerExecutionChain::new(workbook_handler));
//                 } else {
//                     self.own_workbook_handler_execution_chain
//                         .as_mut()
//                         .unwrap()
//                         .add_last(workbook_handler);
//                 }
//             }
//         }

//         if !run_own {
//             self.write_handler_list.push(write_handler);
//         }

//         Ok(())
//     }

//     /// Sorts and cleans up the handler list, removing duplicates.
//     ///
//     /// # Arguments
//     /// * `handler_list` - The handler list to process
//     /// * `run_own` - Whether this is for the own (annotation-based) chain
//     fn sort_and_clear_up_handler(
//         &mut self,
//         handler_list: &mut Vec<Arc<dyn WriteHandler>>,
//         run_own: bool,
//     ) -> Result<()> {
//         // Sort handlers by order
//         let mut order_handler_map: BTreeMap<i32, Vec<Arc<dyn WriteHandler>>> = BTreeMap::new();

//         for handler in handler_list.drain(..) {
//             let order = handler.order();
//             order_handler_map
//                 .entry(order)
//                 .or_insert_with(Vec::new)
//                 .push(handler);
//         }

//         // Clean up duplicates
//         let mut already_existed_handler_set = HashSet::new();
//         let mut clean_up_handler_list = Vec::new();

//         for handlers in order_handler_map.values() {
//             for handler in handlers {
//                 // Check for duplicate handlers implementing NotRepeatExecutor
//                 if let Some(not_repeat_executor) =
//                     handler.as_any().downcast_ref::<dyn NotRepeatExecutor>()
//                 {
//                     let unique_value = not_repeat_executor.unique_value();
//                     if already_existed_handler_set.contains(&unique_value) {
//                         continue;
//                     }
//                     already_existed_handler_set.insert(unique_value);
//                 }
//                 clean_up_handler_list.push(handler.clone());
//             }
//         }

//         // Build chains
//         if !run_own {
//             self.write_handler_list.clear();
//         }

//         for handler in clean_up_handler_list {
//             self.build_chain(handler, run_own)?;
//         }

//         Ok(())
//     }
// }

// impl AbstractWriteHolder for DefaultWriteHolder {
//     fn need_head(&self) -> bool {
//         self.need_head
//     }

//     fn set_need_head(&mut self, need_head: bool) {
//         self.need_head = need_head;
//     }

//     fn relative_head_row_index(&self) -> i32 {
//         self.relative_head_row_index
//     }

//     fn set_relative_head_row_index(&mut self, index: i32) {
//         self.relative_head_row_index = index;
//     }

//     fn excel_write_head_property(&self) -> &ExcelWriteHeadProperty {
//         &self.excel_write_head_property
//     }

//     fn excel_write_head_property_mut(&mut self) -> &mut ExcelWriteHeadProperty {
//         &mut self.excel_write_head_property
//     }

//     fn use_default_style(&self) -> bool {
//         self.use_default_style
//     }

//     fn set_use_default_style(&mut self, use_default_style: bool) {
//         self.use_default_style = use_default_style;
//     }

//     fn automatic_merge_head(&self) -> bool {
//         self.automatic_merge_head
//     }

//     fn set_automatic_merge_head(&mut self, automatic_merge_head: bool) {
//         self.automatic_merge_head = automatic_merge_head;
//     }

//     fn exclude_column_indexes(&self) -> Option<&Vec<i32>> {
//         self.exclude_column_indexes.as_ref()
//     }

//     fn exclude_column_field_names(&self) -> Option<&Vec<String>> {
//         self.exclude_column_field_names.as_ref()
//     }

//     fn include_column_indexes(&self) -> Option<&Vec<i32>> {
//         self.include_column_indexes.as_ref()
//     }

//     fn include_column_field_names(&self) -> Option<&Vec<String>> {
//         self.include_column_field_names.as_ref()
//     }

//     fn order_by_include_column(&self) -> bool {
//         self.order_by_include_column
//     }

//     fn set_order_by_include_column(&mut self, order_by_include_column: bool) {
//         self.order_by_include_column = order_by_include_column;
//     }

//     fn write_handler_list(&self) -> &Vec<Arc<dyn WriteHandler>> {
//         &self.write_handler_list
//     }

//     fn write_handler_list_mut(&mut self) -> &mut Vec<Arc<dyn WriteHandler>> {
//         &mut self.write_handler_list
//     }

//     fn own_workbook_handler_execution_chain(&self) -> Option<&WorkbookHandlerExecutionChain> {
//         self.own_workbook_handler_execution_chain.as_ref()
//     }

//     fn own_workbook_handler_execution_chain_mut(
//         &mut self,
//     ) -> Option<&mut WorkbookHandlerExecutionChain> {
//         self.own_workbook_handler_execution_chain.as_mut()
//     }

//     fn own_sheet_handler_execution_chain(&self) -> Option<&SheetHandlerExecutionChain> {
//         self.own_sheet_handler_execution_chain.as_ref()
//     }

//     fn own_sheet_handler_execution_chain_mut(&mut self) -> Option<&mut SheetHandlerExecutionChain> {
//         self.own_sheet_handler_execution_chain.as_mut()
//     }

//     fn workbook_handler_execution_chain(&self) -> Option<&WorkbookHandlerExecutionChain> {
//         self.workbook_handler_execution_chain.as_ref()
//     }

//     fn workbook_handler_execution_chain_mut(
//         &mut self,
//     ) -> Option<&mut WorkbookHandlerExecutionChain> {
//         self.workbook_handler_execution_chain.as_mut()
//     }

//     fn sheet_handler_execution_chain(&self) -> Option<&SheetHandlerExecutionChain> {
//         self.sheet_handler_execution_chain.as_ref()
//     }

//     fn sheet_handler_execution_chain_mut(&mut self) -> Option<&mut SheetHandlerExecutionChain> {
//         self.sheet_handler_execution_chain.as_mut()
//     }

//     fn row_handler_execution_chain(&self) -> Option<&RowHandlerExecutionChain> {
//         self.row_handler_execution_chain.as_ref()
//     }

//     fn row_handler_execution_chain_mut(&mut self) -> Option<&mut RowHandlerExecutionChain> {
//         self.row_handler_execution_chain.as_mut()
//     }

//     fn cell_handler_execution_chain(&self) -> Option<&CellHandlerExecutionChain> {
//         self.cell_handler_execution_chain.as_ref()
//     }

//     fn cell_handler_execution_chain_mut(&mut self) -> Option<&mut CellHandlerExecutionChain> {
//         self.cell_handler_execution_chain.as_mut()
//     }

//     fn init(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) -> Result<()> {
//         // Check for unsupported features
//         if write_basic_parameter.use_scientific_format().is_some() {
//             return Err(anyhow::anyhow!(
//                 "Currently does not support setting useScientificFormat."
//             ));
//         }

//         // Initialize configuration with inheritance
//         self.init_configuration(write_basic_parameter, parent_abstract_write_holder);

//         // Initialize Excel write head property
//         self.excel_write_head_property = ExcelWriteHeadProperty::new(
//             self,
//             write_basic_parameter.clazz(),
//             write_basic_parameter.head(),
//         )?;

//         // Initialize converters
//         self.init_converters(write_basic_parameter, parent_abstract_write_holder)?;

//         // Initialize handlers
//         self.init_handler(write_basic_parameter, parent_abstract_write_holder)?;

//         Ok(())
//     }

//     fn init_handler(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) -> Result<()> {
//         let mut handler_list = Vec::new();

//         // Initialize annotation-based configuration
//         self.init_annotation_config(&mut handler_list, write_basic_parameter)?;

//         // Add custom handlers
//         if let Some(custom_handlers) = write_basic_parameter.custom_write_handler_list() {
//             handler_list.extend(custom_handlers.iter().cloned());
//         }

//         // Sort and clean up handlers for own chain
//         self.sort_and_clear_up_handler(&mut handler_list, true)?;

//         // Inherit handlers from parent
//         if let Some(parent) = parent_abstract_write_holder {
//             if let Some(parent_handlers) = parent.write_handler_list().first() {
//                 handler_list.extend(parent.write_handler_list().iter().cloned());
//             }
//         } else {
//             // Add default handlers for workbook holder
//             if let Some(workbook_holder) =
//                 self.as_any()
//                     .downcast_ref::<crate::write::metadata::holder::WriteWorkbookHolder>()
//             {
//                 let default_handlers = DefaultWriteHandlerLoader::load_default_handler(
//                     self.use_default_style(),
//                     workbook_holder.excel_type(),
//                 )?;
//                 handler_list.extend(default_handlers);
//             }
//         }

//         // Final sort and cleanup
//         self.sort_and_clear_up_handler(&mut handler_list, false)?;

//         Ok(())
//     }

//     fn init_annotation_config(
//         &mut self,
//         handler_list: &mut Vec<Arc<dyn WriteHandler>>,
//         _write_basic_parameter: &WriteBasicParameter,
//     ) -> Result<()> {
//         // Only process class-based heads
//         if self.excel_write_head_property.head_kind() != HeadKind::Class {
//             return Ok(());
//         }

//         if self.excel_write_head_property.head_class().is_none() {
//             return Ok(());
//         }

//         let head_map = self.excel_write_head_property.head_map();
//         let mut has_column_width = false;

//         // Process each head for annotation-based configuration
//         for head in head_map.values() {
//             if head.column_width_property().is_some() {
//                 has_column_width = true;
//             }

//             // Handle loop merge annotations
//             self.deal_loop_merge(handler_list, head)?;
//         }

//         // Handle column width annotations
//         if has_column_width {
//             self.deal_column_width(handler_list)?;
//         }

//         // Handle style annotations
//         self.deal_style(handler_list)?;

//         // Handle row height annotations
//         self.deal_row_height(handler_list)?;

//         // Handle once absolute merge annotations
//         self.deal_once_absolute_merge(handler_list)?;

//         Ok(())
//     }

//     fn ignore(&self, field_name: Option<&str>, column_index: Option<i32>) -> bool {
//         // Check field name filters
//         if let Some(name) = field_name {
//             if let Some(include_names) = &self.include_column_field_names {
//                 if !include_names.contains(&name.to_string()) {
//                     return true;
//                 }
//             }

//             if let Some(exclude_names) = &self.exclude_column_field_names {
//                 if exclude_names.contains(&name.to_string()) {
//                     return true;
//                 }
//             }
//         }

//         // Check column index filters
//         if let Some(index) = column_index {
//             if let Some(include_indexes) = &self.include_column_indexes {
//                 if !include_indexes.contains(&index) {
//                     return true;
//                 }
//             }

//             if let Some(exclude_indexes) = &self.exclude_column_indexes {
//                 if exclude_indexes.contains(&index) {
//                     return true;
//                 }
//             }
//         }

//         false
//     }
// }

// // Private helper methods for annotation configuration
// impl DefaultWriteHolder {
//     /// Initializes configuration with inheritance from parent.
//     fn init_configuration(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) {
//         // Need head
//         self.need_head = write_basic_parameter
//             .need_head()
//             .or_else(|| parent_abstract_write_holder.map(|p| p.need_head()))
//             .unwrap_or(true);

//         // Relative head row index
//         self.relative_head_row_index = write_basic_parameter
//             .relative_head_row_index()
//             .or_else(|| parent_abstract_write_holder.map(|p| p.relative_head_row_index()))
//             .unwrap_or(0);

//         // Use default style
//         self.use_default_style = write_basic_parameter
//             .use_default_style()
//             .or_else(|| parent_abstract_write_holder.map(|p| p.use_default_style()))
//             .unwrap_or(true);

//         // Automatic merge head
//         self.automatic_merge_head = write_basic_parameter
//             .automatic_merge_head()
//             .or_else(|| parent_abstract_write_holder.map(|p| p.automatic_merge_head()))
//             .unwrap_or(true);

//         // Order by include column
//         self.order_by_include_column = write_basic_parameter
//             .order_by_include_column()
//             .or_else(|| parent_abstract_write_holder.map(|p| p.order_by_include_column()))
//             .unwrap_or(false);

//         // Column filters
//         self.exclude_column_field_names = write_basic_parameter
//             .exclude_column_field_names()
//             .or_else(|| {
//                 parent_abstract_write_holder.and_then(|p| p.exclude_column_field_names().cloned())
//             })
//             .cloned();

//         self.exclude_column_indexes = write_basic_parameter
//             .exclude_column_indexes()
//             .or_else(|| {
//                 parent_abstract_write_holder.and_then(|p| p.exclude_column_indexes().cloned())
//             })
//             .cloned();

//         self.include_column_field_names = write_basic_parameter
//             .include_column_field_names()
//             .or_else(|| {
//                 parent_abstract_write_holder.and_then(|p| p.include_column_field_names().cloned())
//             })
//             .cloned();

//         self.include_column_indexes = write_basic_parameter
//             .include_column_indexes()
//             .or_else(|| {
//                 parent_abstract_write_holder.and_then(|p| p.include_column_indexes().cloned())
//             })
//             .cloned();
//     }

//     /// Initializes converters with inheritance from parent.
//     fn init_converters(
//         &mut self,
//         write_basic_parameter: &WriteBasicParameter,
//         parent_abstract_write_holder: Option<&dyn AbstractWriteHolder>,
//     ) -> Result<()> {
//         if parent_abstract_write_holder.is_none() {
//             // Load default converters for root holder
//             self.converter_map = DefaultConverterLoader::load_default_write_converter()?;
//         } else {
//             // Clone converters from parent
//             let parent = parent_abstract_write_holder.unwrap();
//             // Note: This assumes parent has a way to expose its converter map
//             // We might need to add a method to AbstractWriteHolder for this
//         }

//         // Add custom converters
//         if let Some(custom_converters) = write_basic_parameter.custom_converter_list() {
//             for converter in custom_converters {
//                 let key = ConverterKeyBuild::build_key(converter.support_java_type_key());
//                 self.converter_map.insert(key, converter.clone());
//             }
//         }

//         Ok(())
//     }

//     /// Handles loop merge annotation configuration.
//     fn deal_loop_merge(
//         &self,
//         handler_list: &mut Vec<Arc<dyn WriteHandler>>,
//         head: &Head,
//     ) -> Result<()> {
//         if let Some(loop_merge_property) = head.loop_merge_property() {
//             let strategy = LoopMergeStrategy::new(loop_merge_property.clone(), head.column_index());
//             handler_list.push(Arc::new(strategy));
//         }
//         Ok(())
//     }

//     /// Handles once absolute merge annotation configuration.
//     fn deal_once_absolute_merge(
//         &self,
//         handler_list: &mut Vec<Arc<dyn WriteHandler>>,
//     ) -> Result<()> {
//         if let Some(once_absolute_merge_property) = self
//             .excel_write_head_property
//             .once_absolute_merge_property()
//         {
//             let strategy = OnceAbsoluteMergeStrategy::new(once_absolute_merge_property.clone());
//             handler_list.push(Arc::new(strategy));
//         }
//         Ok(())
//     }

//     /// Handles row height annotation configuration.
//     fn deal_row_height(&self, handler_list: &mut Vec<Arc<dyn WriteHandler>>) -> Result<()> {
//         let head_row_height = self
//             .excel_write_head_property
//             .head_row_height_property()
//             .and_then(|p| p.height());

//         let content_row_height = self
//             .excel_write_head_property
//             .content_row_height_property()
//             .and_then(|p| p.height());

//         if head_row_height.is_some() || content_row_height.is_some() {
//             let strategy = SimpleRowHeightStyleStrategy::new(head_row_height, content_row_height);
//             handler_list.push(Arc::new(strategy));
//         }

//         Ok(())
//     }

//     /// Handles column width annotation configuration.
//     fn deal_column_width(&self, handler_list: &mut Vec<Arc<dyn WriteHandler>>) -> Result<()> {
//         let strategy = Arc::new(AnnotationColumnWidthStyleStrategy);
//         handler_list.push(strategy);
//         Ok(())
//     }

//     /// Handles style annotation configuration.
//     fn deal_style(&self, handler_list: &mut Vec<Arc<dyn WriteHandler>>) -> Result<()> {
//         let strategy = Arc::new(AnnotationStyleStrategy {
//             order: OrderConstant::ANNOTATION_DEFINE_STYLE,
//         });
//         handler_list.push(strategy);
//         Ok(())
//     }
// }

// // Annotation-based strategy implementations
// struct AnnotationStyleStrategy {
//     order: i32,
// }

// impl WriteHandler for AnnotationStyleStrategy {
//     fn order(&self) -> i32 {
//         self.order
//     }

//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
// }

// impl AbstractVerticalCellStyleStrategy for AnnotationStyleStrategy {
//     fn head_cell_style(&self, context: &CellWriteHandlerContext) -> Option<WriteCellStyle> {
//         let head = context.head_data()?;
//         WriteCellStyle::build(head.head_style_property(), head.head_font_property())
//     }

//     fn content_cell_style(&self, context: &CellWriteHandlerContext) -> Option<WriteCellStyle> {
//         let excel_content_property = context.excel_content_property()?;
//         WriteCellStyle::build(
//             excel_content_property.content_style_property(),
//             excel_content_property.content_font_property(),
//         )
//     }
// }

// struct AnnotationColumnWidthStyleStrategy;

// impl WriteHandler for AnnotationColumnWidthStyleStrategy {
//     fn order(&self) -> i32 {
//         OrderConstant::ANNOTATION_DEFINE_COLUMN_WIDTH
//     }

//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }
// }

// impl AbstractHeadColumnWidthStyleStrategy for AnnotationColumnWidthStyleStrategy {
//     fn column_width(&self, head: Option<&Head>, _column_index: i32) -> Option<i32> {
//         let head = head?;
//         head.column_width_property().and_then(|p| p.width())
//     }
// }

// impl Default for DefaultWriteHolder {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// // Implement AbstractHolder for DefaultWriteHolder
// impl AbstractHolder for DefaultWriteHolder {
//     fn clazz(&self) -> Option<&std::any::TypeId> {
//         self.excel_write_head_property.head_class()
//     }

//     fn head(&self) -> Option<&Vec<Head>> {
//         self.excel_write_head_property.head()
//     }

//     fn converter_map(&self) -> &HashMap<TypeId, Arc<dyn Converter>> {
//         &self.converter_map
//     }

//     fn converter_map_mut(&mut self) -> &mut HashMap<TypeId, Arc<dyn Converter>> {
//         &mut self.converter_map
//     }

//     fn set_converter_map(&mut self, converter_map: HashMap<TypeId, Arc<dyn Converter>>) {
//         self.converter_map = converter_map;
//     }

//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
//         self
//     }
// }

// // Implement WriteHolder for DefaultWriteHolder
// impl WriteHolder for DefaultWriteHolder {
//     fn excel_write_head_property(&self) -> &ExcelWriteHeadProperty {
//         &self.excel_write_head_property
//     }

//     fn need_head(&self) -> bool {
//         self.need_head
//     }

//     fn relative_head_row_index(&self) -> i32 {
//         self.relative_head_row_index
//     }

//     fn automatic_merge_head(&self) -> bool {
//         self.automatic_merge_head
//     }

//     fn order_by_include_column(&self) -> bool {
//         self.order_by_include_column
//     }

//     fn include_column_indexes(&self) -> Option<&Vec<i32>> {
//         self.include_column_indexes.as_ref()
//     }

//     fn include_column_field_names(&self) -> Option<&Vec<String>> {
//         self.include_column_field_names.as_ref()
//     }

//     fn exclude_column_indexes(&self) -> Option<&Vec<i32>> {
//         self.exclude_column_indexes.as_ref()
//     }

//     fn exclude_column_field_names(&self) -> Option<&Vec<String>> {
//         self.exclude_column_field_names.as_ref()
//     }

//     fn ignore(&self, field_name: Option<&str>, column_index: Option<i32>) -> bool {
//         AbstractWriteHolder::ignore(self, field_name, column_index)
//     }
// }

// // Provide a builder pattern for easier configuration
// impl DefaultWriteHolder {
//     /// Creates a builder for DefaultWriteHolder.
//     pub fn builder() -> DefaultWriteHolderBuilder {
//         DefaultWriteHolderBuilder::new()
//     }
// }

// /// Builder for DefaultWriteHolder.
// pub struct DefaultWriteHolderBuilder {
//     holder: DefaultWriteHolder,
// }

// impl DefaultWriteHolderBuilder {
//     pub fn new() -> Self {
//         Self {
//             holder: DefaultWriteHolder::new(),
//         }
//     }

//     pub fn need_head(mut self, need_head: bool) -> Self {
//         self.holder.need_head = need_head;
//         self
//     }

//     pub fn relative_head_row_index(mut self, index: i32) -> Self {
//         self.holder.relative_head_row_index = index;
//         self
//     }

//     pub fn use_default_style(mut self, use_default_style: bool) -> Self {
//         self.holder.use_default_style = use_default_style;
//         self
//     }

//     pub fn automatic_merge_head(mut self, automatic_merge_head: bool) -> Self {
//         self.holder.automatic_merge_head = automatic_merge_head;
//         self
//     }

//     pub fn exclude_column_indexes(mut self, indexes: Vec<i32>) -> Self {
//         self.holder.exclude_column_indexes = Some(indexes);
//         self
//     }

//     pub fn exclude_column_field_names(mut self, names: Vec<String>) -> Self {
//         self.holder.exclude_column_field_names = Some(names);
//         self
//     }

//     pub fn include_column_indexes(mut self, indexes: Vec<i32>) -> Self {
//         self.holder.include_column_indexes = Some(indexes);
//         self
//     }

//     pub fn include_column_field_names(mut self, names: Vec<String>) -> Self {
//         self.holder.include_column_field_names = Some(names);
//         self
//     }

//     pub fn order_by_include_column(mut self, order_by_include_column: bool) -> Self {
//         self.holder.order_by_include_column = order_by_include_column;
//         self
//     }

//     pub fn build(self) -> DefaultWriteHolder {
//         self.holder
//     }
// }

// impl Default for DefaultWriteHolderBuilder {
//     fn default() -> Self {
//         Self::new()
//     }
// }
