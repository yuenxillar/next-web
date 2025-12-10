use crate::core::enums::{
    border_style::BorderStyle, fill_pattern_type::FillPatternType,
    horizontal_alignment::HorizontalAlignment, vertical_alignment::VerticalAlignment,
};

/// Custom content cell style
#[derive(Debug, Clone)]
pub struct ContentStyle {
    /// Data format index (must be a valid Excel built-in or custom format).
    /// Built-in formats are defined in Apache POI's `BuiltinFormats`.
    /// Use `-1` to leave unchanged.
    ///
    /// Example: `14` → "m/d/yy", `22` → "m/d/yy h:mm"
    data_format: i16, // short in Java → i16 in Rust

    /// Whether the cell should be hidden
    hidden: Option<bool>,

    /// Whether the cell should be locked (protected)
    locked: Option<bool>,

    /// Enable "Quote Prefix" (forces Excel to treat content as text, like prefixing with ')
    quote_prefix: Option<bool>,

    /// Horizontal alignment of cell content
    horizontal_alignment: Option<HorizontalAlignment>,

    /// Wrap text inside the cell (multi-line)
    wrapped: Option<bool>,

    /// Vertical alignment of cell content
    vertical_alignment: Option<VerticalAlignment>,

    /// Text rotation in degrees.
    /// - HSSF (.xls):  -90 to  90
    /// - XSSF (.xlsx):  0 to 180
    /// Use `-1` to disable rotation.
    rotation: i16,

    /// Number of spaces to indent text (0–15 typically)
    /// Use `-1` to leave unchanged.
    indent: i16,

    /// Left border style
    border_left: Option<BorderStyle>,

    /// Right border style
    border_right: Option<BorderStyle>,

    /// Top border style
    border_top: Option<BorderStyle>,

    /// Bottom border style
    border_bottom: Option<BorderStyle>,

    /// Left border color index (see Apache POI IndexedColors)
    /// Use `-1` to leave unchanged.
    left_border_color: i16,

    /// Right border color index
    right_border_color: i16,

    /// Top border color index
    top_border_color: i16,

    /// Bottom border color index
    bottom_border_color: i16,

    /// Fill pattern type (e.g., SOLID_FOREGROUND)
    fill_pattern_type: Option<FillPatternType>,

    /// Background fill color index
    fill_background_color: i16,

    /// Foreground fill color index (must be set before background for solid fills)
    fill_foreground_color: i16,

    /// Shrink text to fit within cell width
    shrink_to_fit: Option<bool>,
}

impl ContentStyle {
    pub fn get_data_format(&self) -> i16 {
        self.data_format
    }

    pub fn get_hidden(&self) -> Option<bool> {
        self.hidden
    }

    pub fn get_locked(&self) -> Option<bool> {
        self.locked
    }

    pub fn get_quote_prefix(&self) -> Option<bool> {
        self.quote_prefix
    }

    pub fn get_horizontal_alignment(&self) -> Option<&HorizontalAlignment> {
        self.horizontal_alignment.as_ref()
    }

    pub fn get_wrapped(&self) -> Option<bool> {
        self.wrapped
    }

    pub fn get_vertical_alignment(&self) -> Option<&VerticalAlignment> {
        self.vertical_alignment.as_ref()
    }

    pub fn get_rotation(&self) -> i16 {
        self.rotation
    }

    pub fn get_indent(&self) -> i16 {
        self.indent
    }

    pub fn get_border_left(&self) -> Option<&BorderStyle> {
        self.border_left.as_ref()
    }

    pub fn get_border_right(&self) -> Option<&BorderStyle> {
        self.border_right.as_ref()
    }

    pub fn get_border_top(&self) -> Option<&BorderStyle> {
        self.border_top.as_ref()
    }

    pub fn get_border_bottom(&self) -> Option<&BorderStyle> {
        self.border_bottom.as_ref()
    }

    pub fn get_left_border_color(&self) -> i16 {
        self.left_border_color
    }

    pub fn get_right_border_color(&self) -> i16 {
        self.right_border_color
    }

    pub fn get_top_border_color(&self) -> i16 {
        self.top_border_color
    }

    pub fn get_bottom_border_color(&self) -> i16 {
        self.bottom_border_color
    }

    pub fn get_fill_pattern_type(&self) -> Option<&FillPatternType> {
        self.fill_pattern_type.as_ref()
    }

    pub fn get_fill_background_color(&self) -> i16 {
        self.fill_background_color
    }

    pub fn get_fill_foreground_color(&self) -> i16 {
        self.fill_foreground_color
    }

    pub fn get_shrink_to_fit(&self) -> Option<bool> {
        self.shrink_to_fit
    }

    pub fn set_data_format(&mut self, data_format: i16) {
        self.data_format = data_format;
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = Some(hidden);
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = Some(locked);
    }

    pub fn set_quote_prefix(&mut self, quote_prefix: bool) {
        self.quote_prefix = Some(quote_prefix);
    }

    pub fn set_horizontal_alignment(&mut self, horizontal_alignment: HorizontalAlignment) {
        self.horizontal_alignment = Some(horizontal_alignment);
    }

    pub fn set_wrapped(&mut self, wrapped: bool) {
        self.wrapped = Some(wrapped);
    }

    pub fn set_vertical_alignment(&mut self, vertical_alignment: VerticalAlignment) {
        self.vertical_alignment = Some(vertical_alignment);
    }

    pub fn set_rotation(&mut self, rotation: i16) {
        self.rotation = rotation;
    }

    pub fn set_indent(&mut self, indent: i16) {
        self.indent = indent;
    }

    pub fn set_border_left(&mut self, border_left: BorderStyle) {
        self.border_left = Some(border_left);
    }

    pub fn set_border_right(&mut self, border_right: BorderStyle) {
        self.border_right = Some(border_right);
    }

    pub fn set_border_top(&mut self, border_top: BorderStyle) {
        self.border_top = Some(border_top);
    }

    pub fn set_border_bottom(&mut self, border_bottom: BorderStyle) {
        self.border_bottom = Some(border_bottom);
    }

    pub fn set_left_border_color(&mut self, left_border_color: i16) {
        self.left_border_color = left_border_color;
    }

    pub fn set_right_border_color(&mut self, right_border_color: i16) {
        self.right_border_color = right_border_color;
    }

    pub fn set_top_border_color(&mut self, top_border_color: i16) {
        self.top_border_color = top_border_color;
    }

    pub fn set_bottom_border_color(&mut self, bottom_border_color: i16) {
        self.bottom_border_color = bottom_border_color;
    }

    pub fn set_fill_pattern_type(&mut self, fill_pattern_type: FillPatternType) {
        self.fill_pattern_type = Some(fill_pattern_type);
    }

    pub fn set_fill_background_color(&mut self, fill_background_color: i16) {
        self.fill_background_color = fill_background_color;
    }

    pub fn set_fill_foreground_color(&mut self, fill_foreground_color: i16) {
        self.fill_foreground_color = fill_foreground_color;
    }

    pub fn set_shrink_to_fit(&mut self, shrink_to_fit: bool) {
        self.shrink_to_fit = Some(shrink_to_fit);
    }

    pub fn clear_hidden(&mut self) {
        self.hidden = None;
    }

    pub fn clear_locked(&mut self) {
        self.locked = None;
    }

    pub fn clear_quote_prefix(&mut self) {
        self.quote_prefix = None;
    }

    pub fn clear_horizontal_alignment(&mut self) {
        self.horizontal_alignment = None;
    }

    pub fn clear_wrapped(&mut self) {
        self.wrapped = None;
    }

    pub fn clear_vertical_alignment(&mut self) {
        self.vertical_alignment = None;
    }

    pub fn clear_border_left(&mut self) {
        self.border_left = None;
    }

    pub fn clear_border_right(&mut self) {
        self.border_right = None;
    }

    pub fn clear_border_top(&mut self) {
        self.border_top = None;
    }

    pub fn clear_border_bottom(&mut self) {
        self.border_bottom = None;
    }

    pub fn clear_fill_pattern_type(&mut self) {
        self.fill_pattern_type = None;
    }

    pub fn clear_shrink_to_fit(&mut self) {
        self.shrink_to_fit = None;
    }
}

impl Default for ContentStyle {
    fn default() -> Self {
        Self {
            data_format: -1,
            hidden: None,
            locked: None,
            quote_prefix: None,
            horizontal_alignment: None,
            wrapped: None,
            vertical_alignment: None,
            rotation: -1,
            indent: -1,
            border_left: None,
            border_right: None,
            border_top: None,
            border_bottom: None,
            left_border_color: -1,
            right_border_color: -1,
            top_border_color: -1,
            bottom_border_color: -1,
            fill_pattern_type: None,
            fill_background_color: -1,
            fill_foreground_color: -1,
            shrink_to_fit: None,
        }
    }
}
