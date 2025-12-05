use crate::core::{
    converters::converter::Converter,
    metadata::property::{
        date_time_format_property::DateTimeFormatProperty, font_property::FontProperty,
        number_format_property::NumberFormatProperty, style_property::StyleProperty,
    },
};

#[derive(Debug, Clone, Default)]
pub struct ExcelContentProperty {
    field: Option<String>,
    /// Custom defined converters
    converter: Option<Box<dyn Converter>>, // dyn Converter trait object

    /// date time format
    date_time_format_property: Option<DateTimeFormatProperty>,

    /// number format
    number_format_property: Option<NumberFormatProperty>,

    /// Content style
    content_style_property: Option<StyleProperty>,

    /// Content font
    content_font_property: Option<FontProperty>,
}

// 为 ExcelContentProperty 结构体生成的 Getter/Setter 方法

impl ExcelContentProperty {
    pub fn get_field(&self) -> Option<&str> {
        self.field.as_deref()
    }

    pub fn get_converter(&self) -> Option<&dyn Converter> {
        self.converter.as_deref()
    }

    pub fn get_date_time_format_property(&self) -> Option<&DateTimeFormatProperty> {
        self.date_time_format_property.as_ref()
    }

    pub fn get_number_format_property(&self) -> Option<&NumberFormatProperty> {
        self.number_format_property.as_ref()
    }

    pub fn get_content_style_property(&self) -> Option<&StyleProperty> {
        self.content_style_property.as_ref()
    }

    pub fn get_content_font_property(&self) -> Option<&FontProperty> {
        self.content_font_property.as_ref()
    }

    pub fn set_field(&mut self, field: String) {
        self.field = Some(field);
    }

    pub fn set_converter(&mut self, converter: Box<dyn Converter>) {
        self.converter = Some(converter);
    }

    pub fn set_date_time_format_property(
        &mut self,
        date_time_format_property: DateTimeFormatProperty,
    ) {
        self.date_time_format_property = Some(date_time_format_property);
    }

    pub fn set_number_format_property(&mut self, number_format_property: NumberFormatProperty) {
        self.number_format_property = Some(number_format_property);
    }

    pub fn set_content_style_property(&mut self, content_style_property: StyleProperty) {
        self.content_style_property = Some(content_style_property);
    }

    pub fn set_content_font_property(&mut self, content_font_property: FontProperty) {
        self.content_font_property = Some(content_font_property);
    }

    pub fn clear_field(&mut self) {
        self.field = None;
    }

    pub fn clear_converter(&mut self) {
        self.converter = None;
    }

    pub fn clear_date_time_format_property(&mut self) {
        self.date_time_format_property = None;
    }

    pub fn clear_number_format_property(&mut self) {
        self.number_format_property = None;
    }

    pub fn clear_content_style_property(&mut self) {
        self.content_style_property = None;
    }

    pub fn clear_content_font_property(&mut self) {
        self.content_font_property = None;
    }
}
