use std::collections::HashMap;

use crate::core::poi::ss::{
    usermodel::{
        auto_filter::AutoFilter, cell::Cell, cell_range::CellRange, cell_style::CellStyle,
        comment::Comment, data_validation::DataValidation,
        data_validation_helper::DataValidationHelper, drawing::Drawing, footer::Footer,
        header::Header, hyperlink::Hyperlink, page_margin::PageMargin, pane_type::PaneType,
        print_setup::PrintSetup, row::Row, shape::Shape,
        sheet_conditional_formatting::SheetConditionalFormatting, workbook::Workbook,
    },
    util::{
        cell_range_address::CellRangeAddress, cell_range_address_base::CellAddress,
        pane_information::PaneInformation,
    },
};

pub mod sheet_constants {
    // Constants (Rust uses associated constants instead of interface fields)
    const LEFT_MARGIN: u16 = 0;
    const RIGHT_MARGIN: u16 = 1;
    const TOP_MARGIN: u16 = 2;
    const BOTTOM_MARGIN: u16 = 3;
    const HEADER_MARGIN: u16 = 4;
    const FOOTER_MARGIN: u16 = 5;

    const PANE_LOWER_RIGHT: u8 = 0;
    const PANE_UPPER_RIGHT: u8 = 1;
    const PANE_LOWER_LEFT: u8 = 2;
    const PANE_UPPER_LEFT: u8 = 3;
}

pub trait Sheet {
    // Row operations
    fn create_row(&mut self, row_num: u32) -> &dyn Row;
    fn remove_row(&mut self, row: &dyn Row);
    fn get_row(&self, row_num: u32) -> Option<&dyn Row>;
    fn get_physical_number_of_rows(&self) -> u32;
    fn get_first_row_num(&self) -> u32;
    fn get_last_row_num(&self) -> u32;

    // Column operations
    fn set_column_hidden(&mut self, column_index: u32, hidden: bool);
    fn is_column_hidden(&self, column_index: u32) -> bool;
    fn set_right_to_left(&mut self, right_to_left: bool);
    fn is_right_to_left(&self) -> bool;
    fn set_column_width(&mut self, column_index: u32, width: i32);
    fn get_column_width(&self, column_index: u32) -> i32;
    fn get_column_width_in_pixels(&self, column_index: u32) -> f32;
    fn set_default_column_width(&mut self, width: i32);
    fn get_default_column_width(&self) -> i32;
    fn get_column_style(&self, column_index: u32) -> Option<&dyn CellStyle>;
    fn set_default_column_style(&mut self, column_index: u32, style: &dyn CellStyle);

    // Row height operations
    fn get_default_row_height(&self) -> u16;
    fn get_default_row_height_in_points(&self) -> f32;
    fn set_default_row_height(&mut self, height: u16);
    fn set_default_row_height_in_points(&mut self, height: f32);

    // Merged regions
    fn add_merged_region(&mut self, region: &CellRangeAddress) -> i32;
    fn add_merged_region_unsafe(&mut self, region: &CellRangeAddress) -> i32;
    fn validate_merged_regions(&mut self);
    fn remove_merged_region(&mut self, index: u32);
    fn remove_merged_regions(&mut self, indices: &[i32]);
    fn get_num_merged_regions(&self) -> i32;
    fn get_merged_region(&self, index: i32) -> Option<CellRangeAddress>;
    fn get_merged_regions(&self) -> Vec<CellRangeAddress>;

    // Iterator for rows
    fn row_iterator(&self) -> Box<dyn Iterator<Item = &dyn Row> + '_>;

    // Display and print settings
    fn set_force_formula_recalculation(&mut self, value: bool);
    fn get_force_formula_recalculation(&self) -> bool;
    fn set_auto_breaks(&mut self, auto_breaks: bool);
    fn set_display_guts(&mut self, display_guts: bool);
    fn set_display_zeros(&mut self, display_zeros: bool);
    fn is_display_zeros(&self) -> bool;
    fn set_fit_to_page(&mut self, fit_to_page: bool);
    fn set_row_sums_below(&mut self, row_sums_below: bool);
    fn set_row_sums_right(&mut self, row_sums_right: bool);
    fn get_auto_breaks(&self) -> bool;
    fn get_display_guts(&self) -> bool;
    fn get_fit_to_page(&self) -> bool;
    fn get_row_sums_below(&self) -> bool;
    fn get_row_sums_right(&self) -> bool;
    fn is_print_gridlines(&self) -> bool;
    fn set_print_gridlines(&mut self, print_gridlines: bool);
    fn is_print_row_and_column_headings(&self) -> bool;
    fn set_print_row_and_column_headings(&mut self, print_headings: bool);

    // Print and page setup
    fn get_print_setup(&self) -> Option<&dyn PrintSetup>;
    fn get_header(&self) -> Option<&dyn Header>;
    fn get_footer(&self) -> Option<&dyn Footer>;

    // Selection
    fn set_selected(&mut self, selected: bool);
    fn is_selected(&self) -> bool;

    // Margins (using PageMargin enum instead of raw short)
    fn get_margin(&self, margin: PageMargin) -> f64;
    fn set_margin(&mut self, margin: PageMargin, size: f64);

    // Sheet protection
    fn get_protect(&self) -> bool;
    fn protect_sheet(&mut self, password: &str);
    fn get_scenario_protect(&self) -> bool;

    // Zoom and view
    fn set_zoom(&mut self, scale: u32);
    fn get_top_row(&self) -> u16;
    fn get_left_col(&self) -> u16;
    fn show_in_pane(&mut self, top_row: u32, left_col: u32);

    // Row and column shifting
    fn shift_rows(&mut self, start_row: u32, end_row: u32, n: u32);
    fn shift_rows_with_options(
        &mut self,
        start_row: u32,
        end_row: u32,
        n: u32,
        copy_row_height: bool,
        reset_original_row_height: bool,
    );
    fn shift_columns(&mut self, start_col: u32, end_col: u32, n: u32);

    // Freeze and split panes
    fn create_freeze_pane(
        &mut self,
        col_split: u32,
        row_split: u32,
        leftmost_column: u32,
        top_row: u32,
    );
    fn create_freeze_pane_simple(&mut self, col_split: u32, row_split: u32);
    fn create_split_pane(
        &mut self,
        x_split_pos: i32,
        y_split_pos: i32,
        leftmost_column: u32,
        top_row: u32,
        pane_type: PaneType,
    );
    fn get_pane_information(&self) -> Option<PaneInformation>;

    // Display settings
    fn set_display_gridlines(&mut self, show: bool);
    fn is_display_gridlines(&self) -> bool;
    fn set_display_formulas(&mut self, show: bool);
    fn is_display_formulas(&self) -> bool;
    fn set_display_row_col_headings(&mut self, show: bool);
    fn is_display_row_col_headings(&self) -> bool;

    // Row and column breaks
    fn set_row_break(&mut self, row: u32);
    fn is_row_broken(&self, row: u32) -> bool;
    fn remove_row_break(&mut self, row: u32);
    fn get_row_breaks(&self) -> Vec<u32>;

    fn set_column_break(&mut self, column: u32);
    fn is_column_broken(&self, column: u32) -> bool;
    fn remove_column_break(&mut self, column: u32);
    fn get_column_breaks(&self) -> Vec<u32>;

    // Column grouping
    fn set_column_group_collapsed(&mut self, column_index: u32, collapsed: bool);
    fn group_column(&mut self, from_column: u32, to_column: u32);
    fn ungroup_column(&mut self, from_column: u32, to_column: u32);

    // Row grouping
    fn group_row(&mut self, from_row: u32, to_row: u32);
    fn ungroup_row(&mut self, from_row: u32, to_row: u32);
    fn set_row_group_collapsed(&mut self, row_index: u32, collapsed: bool);

    // Auto-sizing
    fn auto_size_column(&mut self, column: u32);
    fn auto_size_column_with_mss(&mut self, column: u32, use_mss: bool);

    // Comments
    fn get_cell_comment(&self, address: &CellAddress) -> Option<&dyn Comment>;
    fn get_cell_comments(&self) -> HashMap<CellAddress, Box<dyn Comment>>;

    // Drawing
    fn get_drawing_patriarch<D: Shape>(&self) -> Option<&dyn Drawing<D>>;
    fn create_drawing_patriarch<D: Shape>(&mut self) -> Box<dyn Drawing<D>>;
    // Workbook and sheet info
    fn get_workbook(&self) -> Option<&dyn Workbook>;
    fn get_sheet_name(&self) -> &str;

    // Array formulas
    fn set_array_formula<C: Cell>(
        &mut self,
        formula: &str,
        range: &CellRangeAddress,
    ) -> Box<dyn CellRange<C>>;
    fn remove_array_formula<C: Cell>(&mut self, cell: &dyn Cell) -> Box<dyn CellRange<C>>;

    // Data validation
    fn get_data_validation_helper(&self) -> Option<&dyn DataValidationHelper>;
    fn get_data_validations(&self) -> Vec<Box<dyn DataValidation>>;
    fn add_validation_data(&mut self, data_validation: Box<dyn DataValidation>);

    // Auto-filter
    fn set_auto_filter(&mut self, range: &CellRangeAddress) -> Option<&dyn AutoFilter>;

    // Conditional formatting
    fn get_sheet_conditional_formatting(&self) -> Option<&dyn SheetConditionalFormatting>;

    // Repeating rows/columns for printing
    fn get_repeating_rows(&self) -> Option<CellRangeAddress>;
    fn get_repeating_columns(&self) -> Option<CellRangeAddress>;
    fn set_repeating_rows(&mut self, rows: Option<&CellRangeAddress>);
    fn set_repeating_columns(&mut self, columns: Option<&CellRangeAddress>);

    // Column outline level
    fn get_column_outline_level(&self, column_index: u32) -> u32;

    // Hyperlinks
    fn get_hyperlink(&self, row: u32, column: u32) -> Option<&dyn Hyperlink>;
    fn get_hyperlink_by_address(&self, address: &CellAddress) -> Option<&dyn Hyperlink>;
    fn get_hyperlink_list(&self) -> Vec<Box<dyn Hyperlink>>;

    // Active cell
    fn get_active_cell(&self) -> Option<CellAddress>;
    fn set_active_cell(&mut self, address: &CellAddress);
}
