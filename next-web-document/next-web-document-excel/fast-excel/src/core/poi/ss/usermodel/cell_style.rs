use std::fmt::Debug;

use next_web_core::{DynClone, clone_trait_object};

use crate::core::{
    enums::{
        border_style::BorderStyle, fill_pattern_type::FillPatternType,
        horizontal_alignment::HorizontalAlignment, vertical_alignment::VerticalAlignment,
    },
    metadata::color::Color,
    poi::ss::usermodel::font::Font,
};

pub trait CellStyle
where
    Self: DynClone,
    Self: Debug,
{
    /// Returns the index of this style in the workbook's style table
    fn get_index(&self) -> u16;

    /// Sets the data format index (e.g. 14 = "yyyy-mm-dd", 2 = "0.00")
    fn set_data_format(&mut self, var1: u16);

    /// Gets the current data format index
    fn get_data_format(&self) -> u16;

    /// Gets the format string (e.g. "yyyy-mm-dd", "0.00%", etc.)
    fn get_data_format_string(&self) -> Option<&str>;

    /// Sets the font for this cell style
    fn set_font(&mut self, font: Box<dyn Font>);

    /// Gets the index of the font used by this style
    fn get_font_index(&self) -> u16;

    /// Sets whether the column containing cells with this style is hidden
    fn set_hidden(&mut self, hidden: bool);

    /// Returns true if the column is hidden
    fn get_hidden(&self) -> bool;

    /// Sets whether cells using this style are locked (protected)
    fn set_locked(&mut self, locked: bool);

    /// Returns true if cells are locked
    fn get_locked(&self) -> bool;

    /// When true, Excel treats content as text even if it looks like a number/formula
    /// Equivalent to prefixing cell value with a single quote (')
    fn set_quote_prefixed(&mut self, quote_prefixed: bool);

    /// Returns true if quote prefix is enabled
    fn get_quote_prefixed(&self) -> bool;

    /// Sets horizontal alignment
    fn set_alignment(&mut self, alignment: HorizontalAlignment);

    /// Gets current horizontal alignment
    fn get_alignment(&self) -> Option<&HorizontalAlignment>;

    /// Enables/disables text wrapping
    fn set_wrap_text(&mut self, wrap: bool);

    /// Returns true if text wrapping is enabled
    fn get_wrap_text(&self) -> bool;

    /// Sets vertical alignment
    fn set_vertical_alignment(&mut self, alignment: VerticalAlignment);

    /// Gets current vertical alignment
    fn get_vertical_alignment(&self) -> Option<&VerticalAlignment>;

    /// Sets text rotation in degrees
    /// HSSF (.xls): -90 to 90
    /// XSSF (.xlsx): 0 to 180 (converted internally)
    fn set_rotation(&mut self, rotation: i16);

    /// Gets current rotation in degrees
    fn get_rotation(&self) -> i16;

    /// Sets left indentation (number of spaces)
    fn set_indention(&mut self, indent: u16);

    /// Gets current indentation
    fn get_indention(&self) -> u16;

    // ──────────────────────────────────────────────
    // Border settings
    // ──────────────────────────────────────────────

    fn set_border_left(&mut self, style: BorderStyle);
    fn get_border_left(&self) -> Option<&BorderStyle>;

    fn set_border_right(&mut self, style: BorderStyle);
    fn get_border_right(&self) -> Option<&BorderStyle>;

    fn set_border_top(&mut self, style: BorderStyle);
    fn get_border_top(&self) -> Option<&BorderStyle>;

    fn set_border_bottom(&mut self, style: BorderStyle);
    fn get_border_bottom(&self) -> Option<&BorderStyle>;

    // ──────────────────────────────────────────────
    // Border colors (IndexedColors index)
    // ──────────────────────────────────────────────

    fn set_left_border_color(&mut self, color: u16);
    fn get_left_border_color(&self) -> u16;

    fn set_right_border_color(&mut self, color: u16);
    fn get_right_border_color(&self) -> u16;

    fn set_top_border_color(&mut self, color: u16);
    fn get_top_border_color(&self) -> u16;

    fn set_bottom_border_color(&mut self, color: u16);
    fn get_bottom_border_color(&self) -> u16;

    // ──────────────────────────────────────────────
    // Fill / Background
    // ──────────────────────────────────────────────

    fn set_fill_pattern(&mut self, pattern: FillPatternType);
    fn get_fill_pattern(&self) -> Option<&FillPatternType>;

    /// Set background fill color using IndexedColors index
    fn set_fill_background_color(&mut self, color: u16);
    fn set_fill_background_color_rgb(&mut self, color: &dyn Color);

    fn get_fill_background_color(&self) -> u16;
    fn get_fill_background_color_rgb(&self) -> Option<&dyn Color>;

    /// Set foreground fill color (used with patterns like SOLID_FOREGROUND)
    fn set_fill_foreground_color(&mut self, color: u16);
    fn set_fill_foreground_color_rgb(&mut self, color: &dyn Color);

    fn get_fill_foreground_color(&self) -> u16;
    fn get_fill_foreground_color_rgb(&self) -> Option<&dyn Color>;

    /// Copy all style attributes from another CellStyle
    fn clone_style_from(&mut self, source: &dyn CellStyle);

    /// Shrink text to fit within cell width
    fn set_shrink_to_fit(&mut self, shrink: bool);
    fn get_shrink_to_fit(&self) -> bool;
}

clone_trait_object!(CellStyle);
