/// Custom content font style annotation (100% equivalent to Java @ContentFontStyle)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFontStyle {
    /// The name for the font (i.e. Arial)
    pub font_name: Option<String>,

    /// Height in the familiar unit of measure - points
    pub font_height_in_points: i16,

    /// Whether to use italics or not
    pub italic: Option<bool>,

    /// Whether to use a strikeout horizontal line through the text or not
    pub strikeout: Option<bool>,

    /// The color for the font
    pub color: i16,

    /// Set normal, super or subscript.
    pub type_offset: i16,

    /// set type of text underlining to use
    pub underline: i8,

    /// Set character-set to use.
    pub charset: i32,

    /// Bold
    pub bold: Option<bool>,
}

impl Default for ContentFontStyle {
    /// Matches exact default values from Java annotation
    #[inline]
    fn default() -> Self {
        Self {
            font_name: None,
            font_height_in_points: -1,
            italic: None,
            strikeout: None,
            color: -1,
            type_offset: -1,
            underline: -1,
            charset: -1,
            bold: None,
        }
    }
}

impl ContentFontStyle {
    #[inline]
    pub fn has_font_name(&self) -> bool {
        self.font_name
            .as_ref()
            .map(|s| !s.is_empty())
            .unwrap_or_default()
    }

    #[inline]
    pub fn is_font_height_set(&self) -> bool {
        self.font_height_in_points >= 0
    }

    #[inline]
    pub fn is_color_set(&self) -> bool {
        self.color >= 0
    }

    #[inline]
    pub fn is_type_offset_set(&self) -> bool {
        self.type_offset >= 0
    }

    #[inline]
    pub fn is_underline_set(&self) -> bool {
        self.underline >= 0
    }

    #[inline]
    pub fn is_charset_set(&self) -> bool {
        self.charset >= 0
    }
}
