use next_web_core::{DynClone, clone_trait_object};

/// Default color (automatic / black)
const COLOR_NORMAL: i16 = i16::MAX; // 32767

/// Red color index (common predefined color)
const COLOR_RED: i16 = 10;

/// No superscript/subscript
const SS_NONE: i16 = 0;
/// Superscript
const SS_SUPER: i16 = 1;
/// Subscript
const SS_SUB: i16 = 2;

/// No underline
const U_NONE: u8 = 0;
/// Single underline
const U_SINGLE: u8 = 1;
/// Double underline
const U_DOUBLE: u8 = 2;
/// Single accounting underline (below text baseline)
const U_SINGLE_ACCOUNTING: u8 = 33;
/// Double accounting underline
const U_DOUBLE_ACCOUNTING: u8 = 34;

/// Character set constants
const ANSI_CHARSET: u8 = 0;
const DEFAULT_CHARSET: u8 = 1;
const SYMBOL_CHARSET: u8 = 2;

/// Conversion factor: 1 point = 20 twips
const TWIPS_PER_POINT: i32 = 20;

pub trait Font
where
    Self: DynClone,
{
    /// Set font name (e.g. "Calibri", "Arial")
    fn set_font_name(&mut self, name: &str);

    /// Get current font name
    fn get_font_name(&self) -> Option<&str>;

    /// Set font height in **twips** (1/20 of a point)
    fn set_font_height(&mut self, height: u16);

    /// Set font height in **points** (more common unit)
    fn set_font_height_in_points(&mut self, points: i16);

    /// Get font height in **twips**
    fn get_font_height(&self) -> u16;

    /// Get font height in **points**
    fn get_font_height_in_points(&self) -> i16;

    /// Set italic
    fn set_italic(&mut self, italic: bool);

    /// Is italic?
    fn get_italic(&self) -> bool;

    /// Set strikeout (strikethrough)
    fn set_strikeout(&mut self, strikeout: bool);

    /// Has strikeout?
    fn get_strikeout(&self) -> bool;

    /// Set bold
    fn set_bold(&mut self, bold: bool);

    /// Is bold?
    fn get_bold(&self) -> bool;
    /// Set color using IndexedColors index
    fn set_color(&mut self, color: u16);

    /// Get color index
    fn get_color(&self) -> u16;

    /// Set type offset (SS_NONE, SS_SUPER, SS_SUB)
    fn set_type_offset(&mut self, offset: i16);

    /// Get type offset
    fn get_type_offset(&self) -> i16;

    /// Set underline style
    fn set_underline(&mut self, underline: u8);

    /// Get underline style
    fn get_underline(&self) -> u8;

    /// Get character set (ANSI, SYMBOL, etc.)
    fn get_char_set(&self) -> u8;

    /// Set character set (byte)
    fn set_char_set_byte(&mut self, charset: u8);

    /// Set character set (int overload)
    fn set_char_set_int(&mut self, charset: i32);

    /// Get font index in workbook
    fn get_index(&self) -> u16;
}

clone_trait_object!(Font);
