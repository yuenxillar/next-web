use std::io;

use crate::core::{
    metadata::csv::csv_data_format::DataFormat,
    poi::ss::{
        formula::evaluation_workbook::EvaluationWorkbook,
        spreadsheet_version::SpreadsheetVersion,
        usermodel::{
            cell_reference_type::CellReferenceType, cell_style::CellStyle,
            creation_helper::CreationHelper, font::Font, name::Name, picture_data::PictureData,
            row::MissingCellPolicy, sheet::Sheet, sheet_visibility::SheetVisibility,
            udff_inder::UDFFinder,
        },
    },
};

/// High level representation of an Excel workbook. This is the first object most users
/// will construct whether they are reading or writing a workbook. It is also the
/// top level object for creating new sheets/etc.
pub trait Workbook<S: Sheet>: std::iter::Iterator<Item = S> {
    /// Extended windows meta file
    const PICTURE_TYPE_EMF: i32 = 2;

    /// Windows Meta File
    const PICTURE_TYPE_WMF: i32 = 3;

    /// Mac PICT format
    const PICTURE_TYPE_PICT: i32 = 4;

    /// JPEG format
    const PICTURE_TYPE_JPEG: i32 = 5;

    /// PNG format
    const PICTURE_TYPE_PNG: i32 = 6;

    /// Device independent bitmap
    const PICTURE_TYPE_DIB: i32 = 7;

    /// Excel silently truncates long sheet names to 31 chars.
    /// This constant is used to ensure uniqueness in the first 31 chars
    const MAX_SENSITIVE_SHEET_NAME_LEN: usize = 31;

    /// Convenience method to get the active sheet. The active sheet is the sheet
    /// which is currently displayed when the workbook is viewed in Excel.
    /// 'Selected' sheet(s) is a distinct concept.
    ///
    /// # Returns
    /// * The index of the active sheet (0-based)
    fn get_active_sheet_index(&self) -> usize;

    /// Convenience method to set the active sheet. The active sheet is the sheet
    /// which is currently displayed when the workbook is viewed in Excel.
    /// 'Selected' sheet(s) is a distinct concept.
    ///
    /// # Arguments
    /// * `sheet_index` - Index of the active sheet (0-based)
    fn set_active_sheet(&mut self, sheet_index: usize);

    /// Gets the first tab that is displayed in the list of tabs in excel.
    ///
    /// # Returns
    /// * The first tab that to display in the list of tabs (0-based)
    fn get_first_visible_tab(&self) -> usize;

    /// Sets the first tab that is displayed in the list of tabs in excel.
    ///
    /// # Arguments
    /// * `sheet_index` - The first tab that to display in the list of tabs (0-based)
    fn set_first_visible_tab(&mut self, sheet_index: usize);

    /// Sets the order of appearance for a given sheet.
    ///
    /// # Arguments
    /// * `sheet_name` - The name of the sheet to reorder
    /// * `pos` - The position that we want to insert the sheet into (0 based)
    fn set_sheet_order(&mut self, sheet_name: &str, pos: usize);

    /// Sets the tab whose data is actually seen when the sheet is opened.
    /// This may be different from the "selected sheet" since excel seems to
    /// allow you to show the data of one sheet when another is seen "selected"
    /// in the tabs (at the bottom).
    ///
    /// # Arguments
    /// * `index` - The index of the sheet to select (0 based)
    fn set_selected_tab(&mut self, index: usize);

    /// Set the sheet name.
    ///
    /// # Arguments
    /// * `sheet` - Sheet number (0 based)
    /// * `name` - New name for the sheet
    ///
    /// # Errors
    /// * Returns error if the name is null or invalid or workbook already contains a sheet with this name
    fn set_sheet_name(&mut self, sheet: usize, name: &str) -> Result<(), String>;

    /// Get the sheet name
    ///
    /// # Arguments
    /// * `sheet` - Sheet number (0 based)
    ///
    /// # Returns
    /// * Sheet name
    fn get_sheet_name(&self, sheet: usize) -> Option<&str>;

    /// Returns the index of the sheet by its name
    ///
    /// # Arguments
    /// * `name` - The sheet name
    ///
    /// # Returns
    /// * Index of the sheet (0 based)
    fn get_sheet_index(&self, name: &str) -> Option<usize>;

    /// Returns the index of the given sheet
    ///
    /// # Arguments
    /// * `sheet` - The sheet to look up
    ///
    /// # Returns
    /// * Index of the sheet (0 based)
    fn get_sheet_index_from_sheet(&self, sheet: &dyn Sheet) -> Option<usize>;

    /// Create a Sheet for this Workbook, adds it to the sheets and returns
    /// the high level representation. Use this to create new sheets.
    ///
    /// # Returns
    /// * Sheet representing the new sheet
    fn create_sheet(&mut self) -> Box<dyn Sheet>;

    /// Create a new sheet for this Workbook and return the high level representation.
    /// Use this to create new sheets.
    ///
    /// # Arguments
    /// * `sheet_name` - The name to set for the sheet
    ///
    /// # Returns
    /// * Sheet representing the new sheet
    ///
    /// # Errors
    /// * Returns error if the name is null or invalid or workbook already contains a sheet with this name
    fn create_sheet_with_name(&mut self, sheet_name: &str) -> Result<Box<dyn Sheet>, String>;

    /// Create a Sheet from an existing sheet in the Workbook.
    ///
    /// # Arguments
    /// * `sheet_num` - Index of the sheet to clone (0-based)
    ///
    /// # Returns
    /// * Sheet representing the cloned sheet
    fn clone_sheet(&mut self, sheet_num: usize) -> Result<Box<dyn Sheet>, String>;

    /// Get the number of spreadsheets in the workbook
    ///
    /// # Returns
    /// * The number of sheets
    fn get_number_of_sheets(&self) -> usize;

    /// Get the Sheet object at the given index.
    ///
    /// # Arguments
    /// * `index` - Index of the sheet number (0-based physical & logical)
    ///
    /// # Returns
    /// * Sheet at the provided index
    fn get_sheet_at(&self, index: usize) -> Option<Box<dyn Sheet>>;

    /// Get sheet with the given name
    ///
    /// # Arguments
    /// * `name` - Name of the sheet
    ///
    /// # Returns
    /// * Sheet with the name provided or `None` if it does not exist
    fn get_sheet(&self, name: &str) -> Option<Box<dyn Sheet>>;

    /// Removes sheet at the given index
    ///
    /// # Arguments
    /// * `index` - Index of the sheet to remove (0-based)
    fn remove_sheet_at(&mut self, index: usize) -> Result<(), String>;

    /// Create a new Font and add it to the workbook's font table
    ///
    /// # Returns
    /// * New font object
    fn create_font(&mut self) -> Box<dyn Font>;

    /// Finds a font that matches the one with the supplied attributes
    ///
    /// # Arguments
    /// * `bold` - Bold attribute
    /// * `color` - Color attribute
    /// * `font_height` - Font height
    /// * `name` - Font name
    /// * `italic` - Italic attribute
    /// * `strikeout` - Strikeout attribute
    /// * `type_offset` - Type offset
    /// * `underline` - Underline attribute
    ///
    /// # Returns
    /// * The font with the matched attributes or `None`
    fn find_font(
        &self,
        bold: bool,
        color: u16,
        font_height: u16,
        name: &str,
        italic: bool,
        strikeout: bool,
        type_offset: u16,
        underline: u8,
    ) -> Option<Box<dyn Font>>;

    /// Get the number of fonts in the font table
    ///
    /// # Returns
    /// * Number of fonts
    fn get_number_of_fonts(&self) -> usize;

    /// Get the font at the given index number
    ///
    /// # Arguments
    /// * `idx` - Index number (0-based)
    ///
    /// # Returns
    /// * Font at the index
    fn get_font_at(&self, idx: usize) -> Option<Box<dyn Font>>;

    /// Create a new Cell style and add it to the workbook's style table
    ///
    /// # Returns
    /// * The new Cell Style object
    ///
    /// # Errors
    /// * Returns error if the number of cell styles exceeded the limit for this type of Workbook
    fn create_cell_style(&mut self) -> Result<Box<dyn CellStyle>, String>;

    /// Get the number of styles the workbook contains
    ///
    /// # Returns
    /// * Count of cell styles
    fn get_num_cell_styles(&self) -> usize;

    /// Get the cell style object at the given index
    ///
    /// # Arguments
    /// * `idx` - Index within the set of styles (0-based)
    ///
    /// # Returns
    /// * CellStyle object at the index
    fn get_cell_style_at(&self, idx: usize) -> Option<Box<dyn CellStyle>>;

    /// Write out this workbook to an OutputStream.
    ///
    /// # Arguments
    /// * `stream` - The output stream to write to
    ///
    /// # Errors
    /// * Returns error if anything can't be written
    fn write(&self, stream: &mut dyn io::Write) -> Result<(), io::Error>;

    /// Close the underlying input resource (File or Stream),
    /// from which the Workbook was read.
    ///
    /// Once this has been called, no further operations, updates or reads should be performed on the Workbook.
    fn close(&mut self) -> Result<(), io::Error>;

    /// Get the total number of defined names in this workbook
    ///
    /// # Returns
    /// * Number of defined names
    fn get_number_of_names(&self) -> usize;

    /// Get a defined name by its name
    ///
    /// # Arguments
    /// * `name` - The name of the defined name
    ///
    /// # Returns
    /// * The defined name with the specified name, or `None` if not found
    fn get_name(&self, name: &str) -> Option<Box<dyn Name>>;

    /// Returns all defined names with the given name.
    ///
    /// # Arguments
    /// * `name` - The name of the defined name
    ///
    /// # Returns
    /// * A list of the defined names with the specified name. An empty list is returned if none is found.
    fn get_names(&self, name: &str) -> Vec<Box<dyn Name>>;

    /// Returns all defined names.
    ///
    /// # Returns
    /// * A list of the defined names. An empty list is returned if none is found.
    fn get_all_names(&self) -> Vec<Box<dyn Name>>;

    /// Creates a new (uninitialised) defined name in this workbook
    ///
    /// # Returns
    /// * New defined name object
    fn create_name(&mut self) -> Box<dyn Name>;

    /// Remove a defined name
    ///
    /// # Arguments
    /// * `name` - The name to remove
    fn remove_name(&mut self, name: Box<dyn Name>) -> Result<(), String>;

    /// Adds the linking required to allow formulas referencing
    /// the specified external workbook to be added to this one.
    ///
    /// # Arguments
    /// * `name` - The name the workbook will be referenced as in formulas
    /// * `workbook` - The open workbook to fetch the link required information from
    ///
    /// # Returns
    /// * Link index
    fn link_external_workbook(&mut self, name: &str, workbook: &dyn Workbook) -> usize;

    /// Sets the print area for the sheet provided
    ///
    /// # Arguments
    /// * `sheet_index` - Zero-based sheet index (0 represents the first sheet)
    /// * `reference` - Valid name Reference for the Print Area
    fn set_print_area(&mut self, sheet_index: usize, reference: &str);

    /// For the convenience of Rust programmers maintaining pointers.
    ///
    /// # Arguments
    /// * `sheet_index` - Zero-based sheet index (0 = First Sheet)
    /// * `start_column` - Column to begin print area
    /// * `end_column` - Column to end the print area
    /// * `start_row` - Row to begin the print area
    /// * `end_row` - Row to end the print area
    fn set_print_area_range(
        &mut self,
        sheet_index: usize,
        start_column: usize,
        end_column: usize,
        start_row: usize,
        end_row: usize,
    );

    /// Retrieves the reference for the print area of the specified sheet,
    /// the sheet name is appended to the reference even if it was not specified.
    ///
    /// # Arguments
    /// * `sheet_index` - Zero-based sheet index (0 represents the first sheet)
    ///
    /// # Returns
    /// * String reference or `None` if no print area has been defined
    fn get_print_area(&self, sheet_index: usize) -> Option<String>;

    /// Delete the print area for the sheet specified
    ///
    /// # Arguments
    /// * `sheet_index` - Zero-based sheet index (0 = First Sheet)
    fn remove_print_area(&mut self, sheet_index: usize);

    /// Retrieves the current policy on what to do when
    /// getting missing or blank cells from a row.
    ///
    /// # Returns
    /// * The missing cell policy
    fn get_missing_cell_policy(&self) -> MissingCellPolicy;

    /// Sets the policy on what to do when
    /// getting missing or blank cells from a row.
    ///
    /// # Arguments
    /// * `policy` - The missing cell policy to set
    fn set_missing_cell_policy(&mut self, policy: MissingCellPolicy);

    /// Returns the instance of DataFormat for this workbook.
    ///
    /// # Returns
    /// * The DataFormat object
    fn create_data_format(&mut self) -> Box<dyn DataFormat>;

    /// Adds a picture to the workbook.
    ///
    /// # Arguments
    /// * `picture_data` - The bytes of the picture
    /// * `format` - The format of the picture
    ///
    /// # Returns
    /// * The index to this picture (1 based)
    fn add_picture(&mut self, picture_data: &[u8], format: i32) -> usize;

    /// Gets all pictures from the Workbook.
    ///
    /// # Returns
    /// * The list of pictures
    fn get_all_pictures(&self) -> Vec<Box<dyn PictureData>>;

    /// Returns an object that handles instantiating concrete
    /// classes of the various instances one needs for HSSF and XSSF.
    ///
    /// # Returns
    /// * Creation helper object
    fn get_creation_helper(&self) -> Option<&dyn CreationHelper>;

    /// Check if the workbook is hidden
    ///
    /// # Returns
    /// * `false` if this workbook is not visible in the GUI
    fn is_hidden(&self) -> bool;

    /// Set workbook hidden flag
    ///
    /// # Arguments
    /// * `hidden_flag` - Pass `false` to make the workbook visible in the GUI
    fn set_hidden(&mut self, hidden_flag: bool);

    /// Check whether a sheet is hidden.
    ///
    /// # Arguments
    /// * `sheet_idx` - Sheet index
    ///
    /// # Returns
    /// * `true` if sheet is hidden
    fn is_sheet_hidden(&self, sheet_idx: usize) -> bool;

    /// Check whether a sheet is very hidden.
    ///
    /// # Arguments
    /// * `sheet_idx` - Sheet index to check
    ///
    /// # Returns
    /// * `true` if sheet is very hidden
    fn is_sheet_very_hidden(&self, sheet_idx: usize) -> bool;

    /// Hide or unhide a sheet.
    ///
    /// # Arguments
    /// * `sheet_idx` - The sheet index (0-based)
    /// * `hidden` - `true` to mark the sheet as hidden, `false` otherwise
    fn set_sheet_hidden(&mut self, sheet_idx: usize, hidden: bool);

    /// Get the visibility (visible, hidden, very hidden) of a sheet in this workbook
    ///
    /// # Arguments
    /// * `sheet_idx` - The index of the sheet
    ///
    /// # Returns
    /// * The sheet visibility
    fn get_sheet_visibility(&self, sheet_idx: usize) -> SheetVisibility;

    /// Hide or unhide a sheet.
    ///
    /// # Arguments
    /// * `sheet_idx` - The sheet index (0-based)
    /// * `visibility` - The sheet visibility to set
    fn set_sheet_visibility(&mut self, sheet_idx: usize, visibility: SheetVisibility);

    /// Register a new toolpack in this workbook.
    ///
    /// # Arguments
    /// * `toolpack` - The toolpack to register
    fn add_tool_pack(&mut self, toolpack: Box<dyn UDFFinder>);

    /// Whether the application shall perform a full recalculation when the workbook is opened.
    ///
    /// # Arguments
    /// * `value` - `true` if the application will perform a full recalculation of
    /// workbook values when the workbook is opened
    fn set_force_formula_recalculation(&mut self, value: bool);

    /// Whether Excel will be asked to recalculate all formulas when the workbook is opened.
    ///
    /// # Returns
    /// * `true` if forced formula recalculation is enabled
    fn get_force_formula_recalculation(&self) -> bool;

    /// Returns the spreadsheet version of this workbook
    ///
    /// # Returns
    /// * SpreadsheetVersion enum
    fn get_spreadsheet_version(&self) -> SpreadsheetVersion;

    /// Adds an OLE package manager object with the given content to the sheet
    ///
    /// # Arguments
    /// * `ole_data` - The payload
    /// * `label` - The label of the payload
    /// * `file_name` - The original filename
    /// * `command` - The command to open the payload
    ///
    /// # Returns
    /// * The index of the added ole object, i.e. the storage id
    ///
    /// # Errors
    /// * Returns error if the object can't be embedded
    fn add_ole_package(
        &mut self,
        ole_data: &[u8],
        label: &str,
        file_name: &str,
        command: &str,
    ) -> Result<usize, io::Error>;

    /// Create an evaluation workbook
    ///
    /// # Returns
    /// * Evaluation workbook
    fn create_evaluation_workbook(&self) -> Box<dyn EvaluationWorkbook>;

    /// Get the type of cell references used
    ///
    /// # Returns
    /// * Cell reference type
    fn get_cell_reference_type(&self) -> CellReferenceType;

    /// Set the type of cell references used
    ///
    /// # Arguments
    /// * `cell_reference_type` - The type of cell references to use
    fn set_cell_reference_type(&mut self, cell_reference_type: CellReferenceType);
}
