// //! Excel write head property for managing header attributes and configurations.
// //!
// //! This struct handles header properties for Excel writing operations, including
// //! style configurations, column widths, merge properties, and header cell ranges.

// use std::any::TypeId;
// use std::collections::{HashMap, HashSet};
// use std::sync::Arc;

// use crate::enums::HeadKind;
// use crate::metadata::{
//     CellRange, ConfigurationHolder, Head, ColumnWidthProperty, ExcelHeadProperty,
//     FontProperty, LoopMergeProperty, OnceAbsoluteMergeProperty, RowHeightProperty, StyleProperty
// };
// use crate::write::annotation::{
//     ColumnWidth as ColumnWidthAnnotation, ContentLoopMerge, ContentRowHeight, HeadFontStyle,
//     HeadRowHeight, HeadStyle, OnceAbsoluteMerge
// };

use std::collections::HashMap;

use crate::core::metadata::head::Head;

// /// Excel write head property defining header attributes for writing operations.
// ///
// /// This struct extends ExcelHeadProperty with additional write-specific properties
// /// such as row heights, merge properties, and style configurations.
// #[derive(Debug, Clone)]
pub struct ExcelWriteHeadProperty {
    //     /// Base Excel head property containing common header information
    //     base_property: Arc<ExcelHeadProperty>,

    //     /// Row height property for header rows
    //     head_row_height_property: Option<RowHeightProperty>,

    //     /// Row height property for content rows
    //     content_row_height_property: Option<RowHeightProperty>,

    //     /// Property for once-absolute merge operations
    //     once_absolute_merge_property: Option<OnceAbsoluteMergeProperty>,

    //     /// Cache of calculated head cell ranges for merging
    //     head_cell_range_list: Option<Vec<CellRange>>,
}

impl ExcelWriteHeadProperty {
    pub fn get_head_map(&self) -> &HashMap<u32, Head> {
        todo!()
    }
}

// impl ExcelWriteHeadProperty {
//     /// Creates a new ExcelWriteHeadProperty from configuration holder, class, and header list.
//     ///
//     /// # Arguments
//     /// * `configuration_holder` - Configuration holder for the write operation
//     /// * `head_clazz` - Optional class type for class-based headers
//     /// * `head` - Optional list of header strings for manual headers
//     ///
//     /// # Returns
//     /// * `Result<Self, ExcelError>` - The created write head property or an error
//     pub fn new(
//         configuration_holder: Arc<ConfigurationHolder>,
//         head_clazz: Option<&TypeId>,
//         head: Option<Vec<Vec<String>>>,
//     ) -> Result<Self, crate::error::ExcelError> {
//         // Create base Excel head property
//         let base_property = ExcelHeadProperty::new(configuration_holder, head_clazz, head)?;

//         let mut property = Self {
//             base_property: Arc::new(base_property),
//             head_row_height_property: None,
//             content_row_height_property: None,
//             once_absolute_merge_property: None,
//             head_cell_range_list: None,
//         };

//         // Only process class-based headers for annotation configurations
//         if property.head_kind() == HeadKind::Class {
//             property.process_class_annotations(head_clazz)?;
//         }

//         Ok(property)
//     }

//     /// Processes class annotations for header properties.
//     ///
//     /// This method extracts and processes annotations from the header class
//     /// to configure row heights, merge properties, and style properties.
//     fn process_class_annotations(
//         &mut self,
//         head_clazz: Option<&TypeId>,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Early return if no class is provided
//         let clazz = match head_clazz {
//             Some(c) => c,
//             None => return Ok(()),
//         };

//         // Get annotation processor
//         let annotation_processor = crate::annotation::AnnotationProcessor::new();

//         // Process row height annotations
//         self.process_row_height_annotations(clazz, &annotation_processor)?;

//         // Process merge annotations
//         self.process_merge_annotations(clazz, &annotation_processor)?;

//         // Process column width and style annotations for each head
//         self.process_head_annotations(clazz, &annotation_processor)?;

//         Ok(())
//     }

//     /// Processes row height annotations from the class.
//     fn process_row_height_annotations(
//         &mut self,
//         clazz: &TypeId,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Process head row height annotation
//         if let Some(head_row_height_annotation) = annotation_processor
//             .get_annotation::<HeadRowHeight>(clazz)
//         {
//             self.head_row_height_property = Some(RowHeightProperty::build(head_row_height_annotation));
//         }

//         // Process content row height annotation
//         if let Some(content_row_height_annotation) = annotation_processor
//             .get_annotation::<ContentRowHeight>(clazz)
//         {
//             self.content_row_height_property = Some(RowHeightProperty::build(content_row_height_annotation));
//         }

//         Ok(())
//     }

//     /// Processes merge annotations from the class.
//     fn process_merge_annotations(
//         &mut self,
//         clazz: &TypeId,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Process once absolute merge annotation
//         if let Some(once_absolute_merge_annotation) = annotation_processor
//             .get_annotation::<OnceAbsoluteMerge>(clazz)
//         {
//             self.once_absolute_merge_property = Some(
//                 OnceAbsoluteMergeProperty::build(once_absolute_merge_annotation)
//             );
//         }

//         Ok(())
//     }

//     /// Processes annotations for each individual head/field.
//     fn process_head_annotations(
//         &mut self,
//         clazz: &TypeId,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Get class-level parent annotations
//         let parent_column_width = annotation_processor.get_annotation::<ColumnWidthAnnotation>(clazz);
//         let parent_head_style = annotation_processor.get_annotation::<HeadStyle>(clazz);
//         let parent_head_font_style = annotation_processor.get_annotation::<HeadFontStyle>(clazz);

//         // Process each head in the head map
//         let head_map = self.base_property.head_map();

//         for (column_index, head) in head_map.iter() {
//             let head = head.clone(); // Clone for mutability

//             // Ensure head exists
//             if head.is_none() {
//                 return Err(crate::error::ExcelError::ConfigurationError(
//                     "Passing in the class and list the head, the two must be the same size.".to_string()
//                 ));
//             }

//             let mut head = head.unwrap();
//             let field_name = head.field_name();

//             // Process column width annotation
//             self.process_column_width_annotation(
//                 &mut head,
//                 field_name,
//                 clazz,
//                 &parent_column_width,
//                 annotation_processor,
//             )?;

//             // Process head style annotation
//             self.process_style_annotation(
//                 &mut head,
//                 field_name,
//                 clazz,
//                 &parent_head_style,
//                 annotation_processor,
//             )?;

//             // Process head font style annotation
//             self.process_font_annotation(
//                 &mut head,
//                 field_name,
//                 clazz,
//                 &parent_head_font_style,
//                 annotation_processor,
//             )?;

//             // Process loop merge annotation
//             self.process_loop_merge_annotation(
//                 &mut head,
//                 field_name,
//                 clazz,
//                 annotation_processor,
//             )?;

//             // Update head in the map
//             self.base_property.update_head(*column_index, head)?;
//         }

//         Ok(())
//     }

//     /// Processes column width annotation for a field.
//     fn process_column_width_annotation(
//         &mut self,
//         head: &mut Head,
//         field_name: &str,
//         clazz: &TypeId,
//         parent_column_width: &Option<Arc<ColumnWidthAnnotation>>,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Try to get field-specific annotation
//         let field_column_width = annotation_processor
//             .get_field_annotation::<ColumnWidthAnnotation>(clazz, field_name);

//         // Use field annotation if available, otherwise use parent annotation
//         let column_width = field_column_width
//             .or_else(|| parent_column_width.clone());

//         if let Some(annotation) = column_width {
//             let column_width_property = ColumnWidthProperty::build(&annotation);
//             head.set_column_width_property(Some(column_width_property));
//         }

//         Ok(())
//     }

//     /// Processes style annotation for a field.
//     fn process_style_annotation(
//         &mut self,
//         head: &mut Head,
//         field_name: &str,
//         clazz: &TypeId,
//         parent_head_style: &Option<Arc<HeadStyle>>,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Try to get field-specific annotation
//         let field_head_style = annotation_processor
//             .get_field_annotation::<HeadStyle>(clazz, field_name);

//         // Use field annotation if available, otherwise use parent annotation
//         let head_style = field_head_style
//             .or_else(|| parent_head_style.clone());

//         if let Some(annotation) = head_style {
//             let style_property = StyleProperty::build(&annotation);
//             head.set_head_style_property(Some(style_property));
//         }

//         Ok(())
//     }

//     /// Processes font annotation for a field.
//     fn process_font_annotation(
//         &mut self,
//         head: &mut Head,
//         field_name: &str,
//         clazz: &TypeId,
//         parent_head_font_style: &Option<Arc<HeadFontStyle>>,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Try to get field-specific annotation
//         let field_head_font_style = annotation_processor
//             .get_field_annotation::<HeadFontStyle>(clazz, field_name);

//         // Use field annotation if available, otherwise use parent annotation
//         let head_font_style = field_head_font_style
//             .or_else(|| parent_head_font_style.clone());

//         if let Some(annotation) = head_font_style {
//             let font_property = FontProperty::build(&annotation);
//             head.set_head_font_property(Some(font_property));
//         }

//         Ok(())
//     }

//     /// Processes loop merge annotation for a field.
//     fn process_loop_merge_annotation(
//         &mut self,
//         head: &mut Head,
//         field_name: &str,
//         clazz: &TypeId,
//         annotation_processor: &crate::annotation::AnnotationProcessor,
//     ) -> Result<(), crate::error::ExcelError> {
//         // Try to get field-specific loop merge annotation
//         if let Some(loop_merge_annotation) = annotation_processor
//             .get_field_annotation::<ContentLoopMerge>(clazz, field_name)
//         {
//             let loop_merge_property = LoopMergeProperty::build(&loop_merge_annotation);
//             head.set_loop_merge_property(Some(loop_merge_property));
//         }

//         Ok(())
//     }

//     /// Calculates all cells that need to be merged in the header.
//     ///
//     /// This method identifies adjacent header cells with the same content
//     /// that should be merged into a single cell.
//     ///
//     /// # Returns
//     /// * `Vec<CellRange>` - List of cell ranges that need to be merged
//     pub fn head_cell_range_list(&mut self) -> Vec<CellRange> {
//         // Return cached result if available
//         if let Some(cached_ranges) = &self.head_cell_range_list {
//             return cached_ranges.clone();
//         }

//         let mut cell_range_list = Vec::new();
//         let mut already_range_set = HashSet::new();

//         // Get head list sorted by column index
//         let head_list = self.sorted_head_list();

//         // Process each head in the list
//         for i in 0..head_list.len() {
//             let head = &head_list[i];
//             let head_name_list = head.head_name_list();

//             // Process each level in the header hierarchy
//             for j in 0..head_name_list.len() {
//                 // Skip already processed cells
//                 let key = format!("{}-{}", i, j);
//                 if already_range_set.contains(&key) {
//                     continue;
//                 }

//                 already_range_set.insert(key.clone());
//                 let head_name = &head_name_list[j];

//                 // Find horizontal merge range
//                 let (last_col, _) = self.find_horizontal_merge_range(
//                     i, j, head_name, &head_list, &mut already_range_set
//                 );

//                 // Find vertical merge range
//                 let last_row = self.find_vertical_merge_range(
//                     i, j, last_col, head_name, &head_list, &mut already_range_set
//                 );

//                 // Create cell range if merge is needed
//                 if j != last_row || i != last_col {
//                     let start_head = &head_list[i];
//                     let end_head = &head_list[last_col];

//                     let cell_range = CellRange::new(
//                         j as i32,            // start row
//                         last_row as i32,     // end row
//                         start_head.column_index(),
//                         end_head.column_index()
//                     );

//                     cell_range_list.push(cell_range);
//                 }
//             }
//         }

//         // Cache the result
//         self.head_cell_range_list = Some(cell_range_list.clone());

//         cell_range_list
//     }

//     /// Finds the horizontal merge range for a header cell.
//     fn find_horizontal_merge_range(
//         &self,
//         start_col: usize,
//         row: usize,
//         head_name: &str,
//         head_list: &[Head],
//         already_range_set: &mut HashSet<String>,
//     ) -> (usize, HashSet<String>) {
//         let mut last_col = start_col;
//         let mut temp_set = HashSet::new();

//         // Check subsequent columns for same header name
//         for k in (start_col + 1)..head_list.len() {
//             let key = format!("{}-{}", k, row);

//             if let Some(next_head) = head_list.get(k) {
//                 let next_head_name = next_head.head_name_list().get(row);

//                 if next_head_name == Some(head_name) && !already_range_set.contains(&key) {
//                     already_range_set.insert(key.clone());
//                     temp_set.insert(key);
//                     last_col = k;
//                 } else {
//                     break;
//                 }
//             }
//         }

//         (last_col, temp_set)
//     }

//     /// Finds the vertical merge range for a header cell.
//     fn find_vertical_merge_range(
//         &self,
//         start_col: usize,
//         start_row: usize,
//         last_col: usize,
//         head_name: &str,
//         head_list: &[Head],
//         already_range_set: &mut HashSet<String>,
//     ) -> usize {
//         let mut last_row = start_row;
//         let mut temp_already_range_set = HashSet::new();

//         // Check subsequent rows for same header name pattern
//         'outer: for k in (start_row + 1)..head_list[start_col].head_name_list().len() {
//             // Check all columns in the current horizontal range
//             for l in start_col..=last_col {
//                 let key = format!("{}-{}", l, k);

//                 if let Some(head) = head_list.get(l) {
//                     let current_head_name = head.head_name_list().get(k);

//                     if current_head_name == Some(head_name) && !already_range_set.contains(&key) {
//                         temp_already_range_set.insert(key);
//                     } else {
//                         break 'outer;
//                     }
//                 }
//             }

//             // If all columns match for this row, extend the vertical range
//             last_row = k;
//             already_range_set.extend(temp_already_range_set.drain());
//         }

//         last_row
//     }

//     /// Gets a sorted list of heads by column index.
//     fn sorted_head_list(&self) -> Vec<Head> {
//         let mut head_list: Vec<Head> = self.base_property.head_map().values().cloned().collect();
//         head_list.sort_by_key(|head| head.column_index());
//         head_list
//     }

//     // Getters for properties

//     /// Gets the head row height property.
//     pub fn head_row_height_property(&self) -> Option<&RowHeightProperty> {
//         self.head_row_height_property.as_ref()
//     }

//     /// Gets the content row height property.
//     pub fn content_row_height_property(&self) -> Option<&RowHeightProperty> {
//         self.content_row_height_property.as_ref()
//     }

//     /// Gets the once absolute merge property.
//     pub fn once_absolute_merge_property(&self) -> Option<&OnceAbsoluteMergeProperty> {
//         self.once_absolute_merge_property.as_ref()
//     }

//     /// Checks if the header has any head rows.
//     pub fn has_head(&self) -> bool {
//         self.base_property.has_head()
//     }

//     /// Gets the head kind (Class, String, or None).
//     pub fn head_kind(&self) -> HeadKind {
//         self.base_property.head_kind()
//     }

//     /// Gets the head class type.
//     pub fn head_class(&self) -> Option<&TypeId> {
//         self.base_property.head_class()
//     }

//     /// Gets the head map (column index to Head mapping).
//     pub fn head_map(&self) -> &HashMap<i32, Head> {
//         self.base_property.head_map()
//     }

//     /// Gets the head map as mutable reference.
//     pub fn head_map_mut(&mut self) -> &mut HashMap<i32, Head> {
//         // Since we need mutable access, we'll need to work with the inner base property
//         // This is a limitation of our current architecture
//         Arc::get_mut(&mut self.base_property)
//             .expect("Cannot get mutable reference to base property")
//             .head_map_mut()
//     }

//     /// Updates a head in the head map.
//     pub fn update_head(&mut self, column_index: i32, head: Head) -> Result<(), crate::error::ExcelError> {
//         Arc::get_mut(&mut self.base_property)
//             .ok_or_else(|| crate::error::ExcelError::ConfigurationError(
//                 "Cannot update head in shared property".to_string()
//             ))?
//             .update_head(column_index, head)
//     }

//     /// Gets the configuration holder.
//     pub fn configuration_holder(&self) -> &Arc<ConfigurationHolder> {
//         self.base_property.configuration_holder()
//     }
// }

// // Implement Deref to delegate to base property for common operations
// impl std::ops::Deref for ExcelWriteHeadProperty {
//     type Target = ExcelHeadProperty;

//     fn deref(&self) -> &Self::Target {
//         &self.base_property
//     }
// }

// // Implement From trait for conversion from ExcelHeadProperty
// impl From<ExcelHeadProperty> for ExcelWriteHeadProperty {
//     fn from(base_property: ExcelHeadProperty) -> Self {
//         Self {
//             base_property: Arc::new(base_property),
//             head_row_height_property: None,
//             content_row_height_property: None,
//             once_absolute_merge_property: None,
//             head_cell_range_list: None,
//         }
//     }
// }

// // Provide a builder pattern for programmatic configuration
// impl ExcelWriteHeadProperty {
//     /// Creates a builder for ExcelWriteHeadProperty.
//     pub fn builder() -> ExcelWriteHeadPropertyBuilder {
//         ExcelWriteHeadPropertyBuilder::new()
//     }
// }

// /// Builder for ExcelWriteHeadProperty.
// pub struct ExcelWriteHeadPropertyBuilder {
//     configuration_holder: Option<Arc<ConfigurationHolder>>,
//     head_clazz: Option<TypeId>,
//     head: Option<Vec<Vec<String>>>,
//     head_row_height_property: Option<RowHeightProperty>,
//     content_row_height_property: Option<RowHeightProperty>,
//     once_absolute_merge_property: Option<OnceAbsoluteMergeProperty>,
// }

// impl ExcelWriteHeadPropertyBuilder {
//     /// Creates a new builder.
//     pub fn new() -> Self {
//         Self {
//             configuration_holder: None,
//             head_clazz: None,
//             head: None,
//             head_row_height_property: None,
//             content_row_height_property: None,
//             once_absolute_merge_property: None,
//         }
//     }

//     /// Sets the configuration holder.
//     pub fn configuration_holder(mut self, holder: Arc<ConfigurationHolder>) -> Self {
//         self.configuration_holder = Some(holder);
//         self
//     }

//     /// Sets the head class.
//     pub fn head_clazz(mut self, clazz: TypeId) -> Self {
//         self.head_clazz = Some(clazz);
//         self
//     }

//     /// Sets the manual header list.
//     pub fn head(mut self, head: Vec<Vec<String>>) -> Self {
//         self.head = Some(head);
//         self
//     }

//     /// Sets the head row height property.
//     pub fn head_row_height_property(mut self, property: RowHeightProperty) -> Self {
//         self.head_row_height_property = Some(property);
//         self
//     }

//     /// Sets the content row height property.
//     pub fn content_row_height_property(mut self, property: RowHeightProperty) -> Self {
//         self.content_row_height_property = Some(property);
//         self
//     }

//     /// Sets the once absolute merge property.
//     pub fn once_absolute_merge_property(mut self, property: OnceAbsoluteMergeProperty) -> Self {
//         self.once_absolute_merge_property = Some(property);
//         self
//     }

//     /// Builds the ExcelWriteHeadProperty.
//     pub fn build(self) -> Result<ExcelWriteHeadProperty, crate::error::ExcelError> {
//         let configuration_holder = self.configuration_holder.ok_or_else(||
//             crate::error::ExcelError::ConfigurationError("Configuration holder is required".to_string())
//         )?;

//         let mut property = ExcelWriteHeadProperty::new(
//             configuration_holder,
//             self.head_clazz.as_ref(),
//             self.head,
//         )?;

//         // Apply custom properties
//         if let Some(row_height) = self.head_row_height_property {
//             property.head_row_height_property = Some(row_height);
//         }

//         if let Some(content_row_height) = self.content_row_height_property {
//             property.content_row_height_property = Some(content_row_height);
//         }

//         if let Some(merge_property) = self.once_absolute_merge_property {
//             property.once_absolute_merge_property = Some(merge_property);
//         }

//         Ok(property)
//     }
// }

// impl Default for ExcelWriteHeadPropertyBuilder {
//     fn default() -> Self {
//         Self::new()
//     }
// }

// // Unit tests
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::metadata::ConfigurationHolderBuilder;

//     #[test]
//     fn test_head_cell_range_list() {
//         // Create a simple configuration holder
//         let config_holder = ConfigurationHolderBuilder::new()
//             .build()
//             .unwrap();

//         // Create a simple header structure for testing
//         let head = vec![
//             vec!["Department".to_string(), "Sales".to_string()],
//             vec!["Department".to_string(), "Sales".to_string()],
//             vec!["Department".to_string(), "Marketing".to_string()],
//         ];

//         let mut property = ExcelWriteHeadProperty::new(
//             Arc::new(config_holder),
//             None,
//             Some(head),
//         ).unwrap();

//         let cell_ranges = property.head_cell_range_list();

//         // Should have one merge range for "Department" across columns 0-2, row 0
//         assert_eq!(cell_ranges.len(), 1);

//         if let Some(range) = cell_ranges.first() {
//             assert_eq!(range.start_row(), 0);
//             assert_eq!(range.end_row(), 0);
//             assert_eq!(range.start_col(), 0);
//             assert_eq!(range.end_col(), 2);
//         }
//     }

//     #[test]
//     fn test_empty_header() {
//         let config_holder = ConfigurationHolderBuilder::new()
//             .build()
//             .unwrap();

//         let property = ExcelWriteHeadProperty::new(
//             Arc::new(config_holder),
//             None,
//             None,
//         ).unwrap();

//         assert!(!property.has_head());
//         assert_eq!(property.head_kind(), HeadKind::None);
//     }
// }
