use crate::core::write::style::{
    content_font_style::ContentFontStyle, head_font_style::HeadFontStyle,
};

#[derive(Debug, Clone, Default)]
pub struct FontProperty {
    /// The name for the font (i. e. Arial)
    font_name: Option<String>,
    /// Height in the familiar unit of measure - points
    font_height_in_points: Option<u16>,
    /// Whether to use italics or not
    italic: Option<bool>,
    /// Whether to use a strikeout horizontal line through the text or not
    strikeout: Option<bool>,
    /// The color for the font
    color: Option<u16>,
    /// Set normal, super or subscript.
    type_offset: Option<i16>,
    /// set type of text underlining to use
    underline: Option<u8>,
    /// Set character-set to use.
    charset: Option<i32>,
    /// Bold
    bold: Option<bool>,
}

impl FontProperty {
    pub fn from_head_font_style(head: &HeadFontStyle) -> Self {
        let mut fp = Self::default();

        if head
            .font_name
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or_default()
        {
            fp.set_font_name(head.font_name.clone().unwrap());
        }

        if head.font_height_in_points >= 0 {
            fp.set_font_height_in_points(head.font_height_in_points as u16);
        }

        if let Some(val) = head.italic {
            fp.set_italic(val);
        }

        if let Some(val) = head.strikeout {
            fp.set_strikeout(val);
        }

        if head.color >= 0 {
            fp.set_color(head.color as u16);
        }

        if head.type_offset >= 0 {
            fp.set_type_offset(head.type_offset as i16);
        }
        if head.underline >= 0 {
            fp.set_underline(head.underline as u8);
        }
        if head.charset >= 0 {
            fp.set_charset(head.charset);
        }
        if let Some(val) = head.bold {
            fp.set_bold(val);
        }

        fp
    }

    pub fn from_content_style(content: &ContentFontStyle) -> Self {
        let mut fp = Self::default();

        if content
            .font_name
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or_default()
        {
            fp.set_font_name(content.font_name.clone().unwrap());
        }
        if content.font_height_in_points >= 0 {
            fp.set_font_height_in_points(content.font_height_in_points as u16);
        }
        if let Some(val) = content.italic {
            fp.set_italic(val);
        }

        if let Some(val) = content.strikeout {
            fp.set_strikeout(val);
        }

        if content.color >= 0 {
            fp.set_color(content.color as u16);
        }
        if content.type_offset >= 0 {
            fp.set_type_offset(content.type_offset as i16);
        }
        if content.underline >= 0 {
            fp.set_underline(content.underline as u8);
        }
        if content.charset >= 0 {
            fp.set_charset(content.charset);
        }

        if let Some(val) = content.bold {
            fp.set_bold(val);
        }

        fp
    }
}

impl FontProperty {
    pub fn get_font_name(&self) -> Option<&str> {
        self.font_name.as_deref()
    }

    pub fn get_font_height_in_points(&self) -> Option<u16> {
        self.font_height_in_points
    }

    pub fn get_italic(&self) -> Option<bool> {
        self.italic
    }

    pub fn get_strikeout(&self) -> Option<bool> {
        self.strikeout
    }

    pub fn get_color(&self) -> Option<u16> {
        self.color
    }

    pub fn get_type_offset(&self) -> Option<i16> {
        self.type_offset
    }

    pub fn get_underline(&self) -> Option<u8> {
        self.underline
    }

    pub fn get_charset(&self) -> Option<i32> {
        self.charset
    }

    pub fn get_bold(&self) -> Option<bool> {
        self.bold
    }

    pub fn set_font_name(&mut self, font_name: String) {
        self.font_name = Some(font_name);
    }

    pub fn set_font_height_in_points(&mut self, font_height_in_points: u16) {
        self.font_height_in_points = Some(font_height_in_points);
    }

    pub fn set_italic(&mut self, italic: bool) {
        self.italic = Some(italic);
    }

    pub fn set_strikeout(&mut self, strikeout: bool) {
        self.strikeout = Some(strikeout);
    }

    pub fn set_color(&mut self, color: u16) {
        self.color = Some(color);
    }

    pub fn set_type_offset(&mut self, type_offset: i16) {
        self.type_offset = Some(type_offset);
    }

    pub fn set_underline(&mut self, underline: u8) {
        self.underline = Some(underline);
    }

    pub fn set_charset(&mut self, charset: i32) {
        self.charset = Some(charset);
    }

    pub fn set_bold(&mut self, bold: bool) {
        self.bold = Some(bold);
    }
}
