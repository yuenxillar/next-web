use crate::core::{
    enums::{
        border_style::BorderStyle, fill_pattern_type::FillPatternType,
        horizontal_alignment::HorizontalAlignment, vertical_alignment::VerticalAlignment,
    },
    metadata::data::data_format_data::DataFormatData,
    write::{
        metadata::style::write_font::WriteFont,
        style::{content_style::ContentStyle, head_style::HeadStyle},
    },
};

#[derive(Debug, Clone, Default)]
pub struct StyleProperty {
    /// Data format for the cell (e.g. "0.00", "yyyy-MM-dd").
    /// Uses POI's built-in or custom format index.
    data_format_data: Option<DataFormatData>,

    /// Font configuration (name, size, bold, color, etc.)
    write_font: Option<WriteFont>,

    /// Whether the cell is hidden
    hidden: Option<bool>,

    /// Whether the cell is locked (protected)
    locked: Option<bool>,

    /// Forces Excel to treat content as text (like prefixing with ')
    quote_prefix: Option<bool>,

    /// Horizontal alignment (LEFT, CENTER, RIGHT, etc.)
    horizontal_alignment: Option<HorizontalAlignment>,

    /// Wrap text inside the cell
    wrapped: Option<bool>,

    /// Vertical alignment (TOP, CENTER, BOTTOM)
    vertical_alignment: Option<VerticalAlignment>,

    /// Text rotation in degrees:
    /// - HSSF: -90 to 90
    /// - XSSF: 0 to 180 (converted internally)
    rotation: Option<i16>,

    /// Number of spaces to indent text
    indent: Option<i16>,

    // ── Borders ─────────────────────────────────────
    border_left: Option<BorderStyle>,
    border_right: Option<BorderStyle>,
    border_top: Option<BorderStyle>,
    border_bottom: Option<BorderStyle>,

    // ── Border Colors (IndexedColors index) ─────────
    left_border_color: Option<u16>,
    right_border_color: Option<u16>,
    top_border_color: Option<u16>,
    bottom_border_color: Option<u16>,

    // ── Fill / Background ───────────────────────────
    fill_pattern_type: Option<FillPatternType>,
    fill_background_color: Option<u16>,
    fill_foreground_color: Option<u16>,

    /// Shrink cell content to fit within column width
    shrink_to_fit: Option<bool>,
}

impl StyleProperty {
    /// Build StyleProperty from a HeadStyle annotation (or None)
    pub fn from_head_style(head_style: &HeadStyle) -> Self {
        let mut style = StyleProperty::default();
        if head_style.get_data_format() >= 0 {
            let mut data_format_data = DataFormatData::default();
            data_format_data.set_index(head_style.get_data_format() as u16);
            style.data_format_data = Some(data_format_data);
        }

        style.hidden = head_style.get_hidden();
        style.locked = head_style.get_locked();
        style.quote_prefix = head_style.get_quote_prefix();
        style.horizontal_alignment = head_style.get_horizontal_alignment().map(Clone::clone);
        style.wrapped = head_style.get_wrapped();
        style.vertical_alignment = head_style.get_vertical_alignment().map(Clone::clone);
        if head_style.get_rotation() >= 0 {
            style.rotation = Some(head_style.get_rotation());
        }
        if head_style.get_indent() >= 0 {
            style.indent = Some(head_style.get_indent());
        }

        style.border_left = head_style.get_border_left().map(Clone::clone);
        style.border_right = head_style.get_border_right().map(Clone::clone);
        style.border_top = head_style.get_border_top().map(Clone::clone);
        style.border_bottom = head_style.get_border_bottom().map(Clone::clone);

        if head_style.get_left_border_color() >= 0 {
            style.left_border_color = Some(head_style.get_left_border_color());
        }
        if head_style.get_right_border_color() >= 0 {
            style.right_border_color = Some(head_style.get_right_border_color());
        }
        if head_style.get_top_border_color() >= 0 {
            style.top_border_color = Some(head_style.get_top_border_color());
        }
        if head_style.get_bottom_border_color() >= 0 {
            style.bottom_border_color = Some(head_style.get_bottom_border_color());
        }

        style.fill_pattern_type = head_style.get_fill_pattern_type().map(Clone::clone);

        if head_style.get_fill_background_color() >= 0 {
            style.fill_background_color = Some(head_style.get_fill_background_color());
        }
        if head_style.get_fill_foreground_color() >= 0 {
            style.fill_foreground_color = Some(head_style.get_fill_foreground_color());
        }

        style.shrink_to_fit = head_style.get_shrink_to_fit();

        style
    }

    /// Build StyleProperty from a ContentStyle annotation (or None)
    pub fn from_content_style(content_style: &ContentStyle) -> Self {
        let mut style = StyleProperty::default();
        if content_style.get_data_format() >= 0 {
            let mut data_format_data = DataFormatData::default();
            data_format_data.set_index(content_style.get_data_format() as u16);
            style.data_format_data = Some(data_format_data);
        }

        style.hidden = content_style.get_hidden();
        style.locked = content_style.get_locked();
        style.quote_prefix = content_style.get_quote_prefix();
        style.horizontal_alignment = content_style.get_horizontal_alignment().map(Clone::clone);
        style.wrapped = content_style.get_wrapped();
        style.vertical_alignment = content_style.get_vertical_alignment().map(Clone::clone);
        if content_style.get_rotation() >= 0 {
            style.rotation = Some(content_style.get_rotation());
        }
        if content_style.get_indent() >= 0 {
            style.indent = Some(content_style.get_indent());
        }

        style.border_left = content_style.get_border_left().map(Clone::clone);
        style.border_right = content_style.get_border_right().map(Clone::clone);
        style.border_top = content_style.get_border_top().map(Clone::clone);
        style.border_bottom = content_style.get_border_bottom().map(Clone::clone);

        if content_style.get_left_border_color() >= 0 {
            style.left_border_color = Some(content_style.get_left_border_color());
        }
        if content_style.get_right_border_color() >= 0 {
            style.right_border_color = Some(content_style.get_right_border_color());
        }
        if content_style.get_top_border_color() >= 0 {
            style.top_border_color = Some(content_style.get_top_border_color());
        }
        if content_style.get_bottom_border_color() >= 0 {
            style.bottom_border_color = Some(content_style.get_bottom_border_color());
        }

        style.fill_pattern_type = content_style.get_fill_pattern_type().map(Clone::clone);

        if content_style.get_fill_background_color() >= 0 {
            style.fill_background_color = Some(content_style.get_fill_background_color());
        }
        if content_style.get_fill_foreground_color() >= 0 {
            style.fill_foreground_color = Some(content_style.get_fill_foreground_color());
        }

        style.shrink_to_fit = content_style.get_shrink_to_fit();

        style
    }
}

// Generated Getter/Setter methods for StyleProperty struct

impl StyleProperty {
    pub fn get_data_format_data(&self) -> Option<&DataFormatData> {
        self.data_format_data.as_ref()
    }

    pub fn get_write_font(&self) -> Option<&WriteFont> {
        self.write_font.as_ref()
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

    pub fn get_rotation(&self) -> Option<i16> {
        self.rotation
    }

    pub fn get_indent(&self) -> Option<i16> {
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

    pub fn get_left_border_color(&self) -> Option<u16> {
        self.left_border_color
    }

    pub fn get_right_border_color(&self) -> Option<u16> {
        self.right_border_color
    }

    pub fn get_top_border_color(&self) -> Option<u16> {
        self.top_border_color
    }

    pub fn get_bottom_border_color(&self) -> Option<u16> {
        self.bottom_border_color
    }

    pub fn get_fill_pattern_type(&self) -> Option<&FillPatternType> {
        self.fill_pattern_type.as_ref()
    }

    pub fn get_fill_background_color(&self) -> Option<u16> {
        self.fill_background_color
    }

    pub fn get_fill_foreground_color(&self) -> Option<u16> {
        self.fill_foreground_color
    }

    pub fn get_shrink_to_fit(&self) -> Option<bool> {
        self.shrink_to_fit
    }

    pub fn set_data_format_data(&mut self, data_format_data: DataFormatData) {
        self.data_format_data = Some(data_format_data);
    }

    pub fn set_write_font(&mut self, write_font: WriteFont) {
        self.write_font = Some(write_font);
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
        self.rotation = Some(rotation);
    }

    pub fn set_indent(&mut self, indent: i16) {
        self.indent = Some(indent);
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

    pub fn set_left_border_color(&mut self, left_border_color: u16) {
        self.left_border_color = Some(left_border_color);
    }

    pub fn set_right_border_color(&mut self, right_border_color: u16) {
        self.right_border_color = Some(right_border_color);
    }

    pub fn set_top_border_color(&mut self, top_border_color: u16) {
        self.top_border_color = Some(top_border_color);
    }

    pub fn set_bottom_border_color(&mut self, bottom_border_color: u16) {
        self.bottom_border_color = Some(bottom_border_color);
    }

    pub fn set_fill_pattern_type(&mut self, fill_pattern_type: FillPatternType) {
        self.fill_pattern_type = Some(fill_pattern_type);
    }

    pub fn set_fill_background_color(&mut self, fill_background_color: u16) {
        self.fill_background_color = Some(fill_background_color);
    }

    pub fn set_fill_foreground_color(&mut self, fill_foreground_color: u16) {
        self.fill_foreground_color = Some(fill_foreground_color);
    }

    pub fn set_shrink_to_fit(&mut self, shrink_to_fit: bool) {
        self.shrink_to_fit = Some(shrink_to_fit);
    }
}
