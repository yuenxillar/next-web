/// Font when writing
#[derive(Debug, Default, Clone)]
pub struct WriteFont {
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

    /// set type of text underlining
    underline: Option<u8>,

    /// Set character-set to use.
    charset: Option<i32>,

    /// Bold
    bold: Option<bool>,
}

impl WriteFont {
    pub fn merge(source: &Self, target: &mut Self) {
        if let Some(name) = source.font_name.as_ref().filter(|s| !s.trim().is_empty()) {
            target.font_name = Some(name.clone());
        }
        if let Some(v) = source.font_height_in_points {
            target.font_height_in_points = Some(v);
        }
        if let Some(v) = source.italic {
            target.italic = Some(v);
        }
        if let Some(v) = source.strikeout {
            target.strikeout = Some(v);
        }
        if let Some(v) = source.color {
            target.color = Some(v);
        }
        if let Some(v) = source.type_offset {
            target.type_offset = Some(v);
        }
        if let Some(v) = source.underline {
            target.underline = Some(v);
        }
        if let Some(v) = source.charset {
            target.charset = Some(v);
        }
        if let Some(v) = source.bold {
            target.bold = Some(v);
        }
    }
}

impl WriteFont {
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

    pub fn clear_font_name(&mut self) {
        self.font_name = None;
    }

    pub fn clear_font_height_in_points(&mut self) {
        self.font_height_in_points = None;
    }

    pub fn clear_italic(&mut self) {
        self.italic = None;
    }

    pub fn clear_strikeout(&mut self) {
        self.strikeout = None;
    }

    pub fn clear_color(&mut self) {
        self.color = None;
    }

    pub fn clear_type_offset(&mut self) {
        self.type_offset = None;
    }

    pub fn clear_underline(&mut self) {
        self.underline = None;
    }

    pub fn clear_charset(&mut self) {
        self.charset = None;
    }

    pub fn clear_bold(&mut self) {
        self.bold = None;
    }
}
