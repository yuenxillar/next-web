use crate::core::{
    enums::{
        border_style::BorderStyle, fill_pattern_type::FillPatternType,
        horizontal_alignment::HorizontalAlignment, vertical_alignment::VerticalAlignment,
    },
    metadata::{
        data::data_format_data::DataFormatData,
        property::{font_property::FontProperty, style_property::StyleProperty},
    },
    write::metadata::style::write_font::WriteFont,
};

#[derive(Debug, Clone, Default)]
pub struct WriteCellStyle {
    /// Set the data format (must be a valid format). Built in formats are defined at BuiltinFormats.
    data_format_data: Option<DataFormatData>,
    /// Set the font for this style
    write_font: Option<WriteFont>,
    /// Set the cell's using this style to be hidden
    hidden: Option<bool>,
    /// Set the cell's using this style to be locked
    locked: Option<bool>,
    /// Turn on or off "Quote Prefix" or "123 Prefix" for the style,
    /// which is used to tell Excel that the thing which looks like a number or a formula shouldn't be treated as on.
    /// Turning this on is somewhat (but not completely, see IgnoredErrorType) like prefixing the cell value with a ' in Excel
    quote_prefix: Option<bool>,
    /// Set the type of horizontal alignment for the cell
    horizontal_alignment: Option<HorizontalAlignment>,
    /// Set whether the text should be wrapped.
    /// Setting this flag to true make all content visible within a cell by displaying it on multiple lines
    wrapped: Option<bool>,
    /// Set the type of vertical alignment for the cell
    vertical_alignment: Option<VerticalAlignment>,

    /// Set the degree of rotation for the text in the cell. Note: HSSF uses values from -90 to 90 degrees, whereas XSSF uses values from 0 to 180 degrees.
    /// The implementations of this method will map between these two value-ranges accordingly,
    /// however the corresponding getter is returning values in the range mandated by the current type of Excel file-format that this CellStyle is applied to.
    rotation: Option<i16>,
    /// Set the number of spaces to indent the text in the cell
    indent: Option<i16>,

    /// Set the type of border to use for the left border of the cell
    border_left: Option<BorderStyle>,
    /// Set the type of border to use for the right border of the cell
    border_right: Option<BorderStyle>,
    /// Set the type of border to use for the top border of the cell
    border_top: Option<BorderStyle>,
    /// Set the type of border to use for the bottom border of the cell
    border_bottom: Option<BorderStyle>,

    /// Set the color to use for the left border
    left_border_color: Option<u16>,
    /// Set the color to use for the right border
    right_border_color: Option<u16>,
    /// Set the color to use for the top border
    top_border_color: Option<u16>,
    /// Set the color to use for the bottom border
    bottom_border_color: Option<u16>,

    /// Setting to one fills the cell with the foreground color... No idea about other values
    fill_pattern_type: Option<FillPatternType>,
    /// Set the background fill color.
    fill_background_color: Option<u16>,
    /// Set the foreground fill color Note: Ensure Foreground color is set prior to background color.
    fill_foreground_color: Option<u16>,
    /// Controls if the Cell should be auto-sized to shrink to fit if the text is too long
    shrink_to_fit: Option<bool>,
}

impl WriteCellStyle {
    /// 把 source 中非空的字段合并到 target（不修改 source）
    pub fn merge(source: &Self, target: &mut Self) {
        if source.data_format_data.is_some() {
            match &mut target.data_format_data {
                Some(target_df) => {
                    if let Some(source_df) = &source.data_format_data {
                        DataFormatData::merge(source_df, target_df);
                    }
                }
                None => target.data_format_data = source.data_format_data.clone(),
            }
        }

        if source.write_font.is_some() {
            match &mut target.write_font {
                Some(target_font) => {
                    if let Some(source_font) = &source.write_font {
                        WriteFont::merge(source_font, target_font);
                    }
                }
                None => target.write_font = source.write_font.clone(),
            }
        }

        macro_rules! merge_field {
            ($field:ident) => {
                if source.$field.is_some() {
                    target.$field = source.$field;
                }
            };
        }

        merge_field!(hidden);
        merge_field!(locked);
        merge_field!(quote_prefix);
        merge_field!(horizontal_alignment);
        merge_field!(wrapped);
        merge_field!(vertical_alignment);
        merge_field!(rotation);
        merge_field!(indent);
        merge_field!(border_left);
        merge_field!(border_right);
        merge_field!(border_top);
        merge_field!(border_bottom);
        merge_field!(left_border_color);
        merge_field!(right_border_color);
        merge_field!(top_border_color);
        merge_field!(bottom_border_color);
        merge_field!(fill_pattern_type);
        merge_field!(fill_background_color);
        merge_field!(fill_foreground_color);
        merge_field!(shrink_to_fit);
    }

    /// 从 StyleProperty + FontProperty 构建 WriteCellStyle（返回 None 当两者都为 None）
    pub fn build(style_property: &StyleProperty, font_property: &FontProperty) -> Self {
        let mut style = Self::default();

        Self::apply_style_property(style_property, &mut style);
        Self::apply_font_property(font_property, &mut style);

        style
    }

    fn apply_font_property(font_prop: &FontProperty, target: &mut Self) {
        let write_font = target.write_font.get_or_insert_default();

        if let Some(name) = font_prop.get_font_name().filter(|s| !s.is_empty()) {
            write_font.set_font_name(name.to_string());
        }

        if let Some(val) = font_prop.get_font_height_in_points() {
            write_font.set_font_height_in_points(val);
        }
        if let Some(val) = font_prop.get_italic() {
            write_font.set_italic(val);
        }
        if let Some(val) = font_prop.get_strikeout() {
            write_font.set_strikeout(val);
        }
        if let Some(val) = font_prop.get_color() {
            write_font.set_color(val);
        }
        if let Some(val) = font_prop.get_type_offset() {
            write_font.set_type_offset(val);
        }
        if let Some(val) = font_prop.get_underline() {
            write_font.set_underline(val);
        }
        if let Some(val) = font_prop.get_charset() {
            write_font.set_charset(val);
        }
        if let Some(val) = font_prop.get_bold() {
            write_font.set_bold(val);
        }
    }

    fn apply_style_property(style_prop: &StyleProperty, target: &mut Self) {
        if let Some(df) = style_prop.get_data_format_data() {
            match &mut target.data_format_data {
                Some(target_df) => DataFormatData::merge(df, target_df),
                None => target.data_format_data = Some(df.clone()),
            }
        }

        macro_rules! apply_opt {
            ($getter:ident, $setter:ident) => {
                if let Some(v) = style_prop.$getter() {
                    target.$setter(Some(v));
                }
            };
        }

        apply_opt!(hidden, set_hidden);
        apply_opt!(locked, set_locked);
        apply_opt!(quote_prefix, set_quote_prefix);
        apply_opt!(horizontal_alignment, set_horizontal_alignment);
        apply_opt!(wrapped, set_wrapped);
        apply_opt!(vertical_alignment, set_vertical_alignment);
        apply_opt!(rotation, set_rotation);
        apply_opt!(indent, set_indent);
        apply_opt!(border_left, set_border_left);
        apply_opt!(border_right, set_border_right);
        apply_opt!(border_top, set_border_top);
        apply_opt!(border_bottom, set_border_bottom);
        apply_opt!(left_border_color, set_left_border_color);
        apply_opt!(right_border_color, set_right_border_color);
        apply_opt!(top_border_color, set_top_border_color);
        apply_opt!(bottom_border_color, set_bottom_border_color);
        apply_opt!(fill_pattern_type, set_fill_pattern_type);
        apply_opt!(fill_background_color, set_fill_background_color);
        apply_opt!(fill_foreground_color, set_fill_foreground_color);
        apply_opt!(shrink_to_fit, set_shrink_to_fit);
    }
}
