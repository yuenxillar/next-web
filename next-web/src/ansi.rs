//! ANSI escape codes, exposed as properties so that they can be used as
//! placeholders in a banner, for example `${AnsiColor.BRIGHT_GREEN}`.

use std::collections::HashMap;
use std::fmt;

use next_web_core::env::{BasePropertySource, PropertySource};

/// Name of the property source that exposes the ANSI escape codes.
pub const ANSI_PROPERTY_SOURCE_NAME: &str = "ansi";

/// A foreground color, exposed as the `AnsiColor.<NAME>` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnsiColor {
    /// The default foreground color.
    Default,
    /// Black.
    Black,
    /// Red.
    Red,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Blue.
    Blue,
    /// Magenta.
    Magenta,
    /// Cyan.
    Cyan,
    /// White.
    White,
    /// Bright black (gray).
    BrightBlack,
    /// Bright red.
    BrightRed,
    /// Bright green.
    BrightGreen,
    /// Bright yellow.
    BrightYellow,
    /// Bright blue.
    BrightBlue,
    /// Bright magenta.
    BrightMagenta,
    /// Bright cyan.
    BrightCyan,
    /// Bright white.
    BrightWhite,
}

impl AnsiColor {
    /// All foreground colors.
    pub const VALUES: [AnsiColor; 17] = [
        AnsiColor::Default,
        AnsiColor::Black,
        AnsiColor::Red,
        AnsiColor::Green,
        AnsiColor::Yellow,
        AnsiColor::Blue,
        AnsiColor::Magenta,
        AnsiColor::Cyan,
        AnsiColor::White,
        AnsiColor::BrightBlack,
        AnsiColor::BrightRed,
        AnsiColor::BrightGreen,
        AnsiColor::BrightYellow,
        AnsiColor::BrightBlue,
        AnsiColor::BrightMagenta,
        AnsiColor::BrightCyan,
        AnsiColor::BrightWhite,
    ];

    /// Returns the name used in property keys, for example `"BRIGHT_RED"`.
    pub fn name(self) -> &'static str {
        match self {
            AnsiColor::Default => "DEFAULT",
            AnsiColor::Black => "BLACK",
            AnsiColor::Red => "RED",
            AnsiColor::Green => "GREEN",
            AnsiColor::Yellow => "YELLOW",
            AnsiColor::Blue => "BLUE",
            AnsiColor::Magenta => "MAGENTA",
            AnsiColor::Cyan => "CYAN",
            AnsiColor::White => "WHITE",
            AnsiColor::BrightBlack => "BRIGHT_BLACK",
            AnsiColor::BrightRed => "BRIGHT_RED",
            AnsiColor::BrightGreen => "BRIGHT_GREEN",
            AnsiColor::BrightYellow => "BRIGHT_YELLOW",
            AnsiColor::BrightBlue => "BRIGHT_BLUE",
            AnsiColor::BrightMagenta => "BRIGHT_MAGENTA",
            AnsiColor::BrightCyan => "BRIGHT_CYAN",
            AnsiColor::BrightWhite => "BRIGHT_WHITE",
        }
    }

    /// Returns the ANSI code of this color, for example `"31"`.
    pub fn code(self) -> &'static str {
        match self {
            AnsiColor::Default => "39",
            AnsiColor::Black => "30",
            AnsiColor::Red => "31",
            AnsiColor::Green => "32",
            AnsiColor::Yellow => "33",
            AnsiColor::Blue => "34",
            AnsiColor::Magenta => "35",
            AnsiColor::Cyan => "36",
            AnsiColor::White => "37",
            AnsiColor::BrightBlack => "90",
            AnsiColor::BrightRed => "91",
            AnsiColor::BrightGreen => "92",
            AnsiColor::BrightYellow => "93",
            AnsiColor::BrightBlue => "94",
            AnsiColor::BrightMagenta => "95",
            AnsiColor::BrightCyan => "96",
            AnsiColor::BrightWhite => "97",
        }
    }
}

/// A background color, exposed as the `AnsiBackground.<NAME>` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnsiBackground {
    /// The default background color.
    Default,
    /// Black.
    Black,
    /// Red.
    Red,
    /// Green.
    Green,
    /// Yellow.
    Yellow,
    /// Blue.
    Blue,
    /// Magenta.
    Magenta,
    /// Cyan.
    Cyan,
    /// White.
    White,
    /// Bright black (gray).
    BrightBlack,
    /// Bright red.
    BrightRed,
    /// Bright green.
    BrightGreen,
    /// Bright yellow.
    BrightYellow,
    /// Bright blue.
    BrightBlue,
    /// Bright magenta.
    BrightMagenta,
    /// Bright cyan.
    BrightCyan,
    /// Bright white.
    BrightWhite,
}

impl AnsiBackground {
    /// All background colors.
    pub const VALUES: [AnsiBackground; 17] = [
        AnsiBackground::Default,
        AnsiBackground::Black,
        AnsiBackground::Red,
        AnsiBackground::Green,
        AnsiBackground::Yellow,
        AnsiBackground::Blue,
        AnsiBackground::Magenta,
        AnsiBackground::Cyan,
        AnsiBackground::White,
        AnsiBackground::BrightBlack,
        AnsiBackground::BrightRed,
        AnsiBackground::BrightGreen,
        AnsiBackground::BrightYellow,
        AnsiBackground::BrightBlue,
        AnsiBackground::BrightMagenta,
        AnsiBackground::BrightCyan,
        AnsiBackground::BrightWhite,
    ];

    /// Returns the name used in property keys, for example `"BRIGHT_RED"`.
    pub fn name(self) -> &'static str {
        match self {
            AnsiBackground::Default => "DEFAULT",
            AnsiBackground::Black => "BLACK",
            AnsiBackground::Red => "RED",
            AnsiBackground::Green => "GREEN",
            AnsiBackground::Yellow => "YELLOW",
            AnsiBackground::Blue => "BLUE",
            AnsiBackground::Magenta => "MAGENTA",
            AnsiBackground::Cyan => "CYAN",
            AnsiBackground::White => "WHITE",
            AnsiBackground::BrightBlack => "BRIGHT_BLACK",
            AnsiBackground::BrightRed => "BRIGHT_RED",
            AnsiBackground::BrightGreen => "BRIGHT_GREEN",
            AnsiBackground::BrightYellow => "BRIGHT_YELLOW",
            AnsiBackground::BrightBlue => "BRIGHT_BLUE",
            AnsiBackground::BrightMagenta => "BRIGHT_MAGENTA",
            AnsiBackground::BrightCyan => "BRIGHT_CYAN",
            AnsiBackground::BrightWhite => "BRIGHT_WHITE",
        }
    }

    /// Returns the ANSI code of this color, for example `"41"`.
    pub fn code(self) -> &'static str {
        match self {
            AnsiBackground::Default => "49",
            AnsiBackground::Black => "40",
            AnsiBackground::Red => "41",
            AnsiBackground::Green => "42",
            AnsiBackground::Yellow => "43",
            AnsiBackground::Blue => "44",
            AnsiBackground::Magenta => "45",
            AnsiBackground::Cyan => "46",
            AnsiBackground::White => "47",
            AnsiBackground::BrightBlack => "100",
            AnsiBackground::BrightRed => "101",
            AnsiBackground::BrightGreen => "102",
            AnsiBackground::BrightYellow => "103",
            AnsiBackground::BrightBlue => "104",
            AnsiBackground::BrightMagenta => "105",
            AnsiBackground::BrightCyan => "106",
            AnsiBackground::BrightWhite => "107",
        }
    }
}

/// A text style, exposed as the `AnsiStyle.<NAME>` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnsiStyle {
    /// Normal text, which also resets the previously applied styles.
    Normal,
    /// Bold text.
    Bold,
    /// Faint text.
    Faint,
    /// Italic text.
    Italic,
    /// Underlined text.
    Underline,
}

impl AnsiStyle {
    /// All styles.
    pub const VALUES: [AnsiStyle; 5] = [
        AnsiStyle::Normal,
        AnsiStyle::Bold,
        AnsiStyle::Faint,
        AnsiStyle::Italic,
        AnsiStyle::Underline,
    ];

    /// Returns the name used in property keys, for example `"BOLD"`.
    pub fn name(self) -> &'static str {
        match self {
            AnsiStyle::Normal => "NORMAL",
            AnsiStyle::Bold => "BOLD",
            AnsiStyle::Faint => "FAINT",
            AnsiStyle::Italic => "ITALIC",
            AnsiStyle::Underline => "UNDERLINE",
        }
    }

    /// Returns the ANSI code of this style, for example `"1"`.
    pub fn code(self) -> &'static str {
        match self {
            AnsiStyle::Normal => "0",
            AnsiStyle::Bold => "1",
            AnsiStyle::Faint => "2",
            AnsiStyle::Italic => "3",
            AnsiStyle::Underline => "4",
        }
    }
}

/// A single ANSI escape code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnsiElement {
    /// A foreground color, for example `AnsiColor.RED`.
    Color(AnsiColor),
    /// A background color, for example `AnsiBackground.RED`.
    Background(AnsiBackground),
    /// A style, for example `AnsiStyle.BOLD`.
    Style(AnsiStyle),
    /// An 8-bit foreground color, for example `AnsiColor.208`.
    EightBitColor(u8),
    /// An 8-bit background color, for example `AnsiBackground.208`.
    EightBitBackground(u8),
}

impl AnsiElement {
    /// Returns the ANSI code of this element, for example `"31"` or `"38;5;208"`.
    pub fn code(self) -> String {
        match self {
            AnsiElement::Color(color) => color.code().to_string(),
            AnsiElement::Background(background) => background.code().to_string(),
            AnsiElement::Style(style) => style.code().to_string(),
            AnsiElement::EightBitColor(code) => format!("38;5;{code}"),
            AnsiElement::EightBitBackground(code) => format!("48;5;{code}"),
        }
    }

    /// Returns the escape sequence selecting this element, for example
    /// `"\u{1b}[31m"`.
    pub fn escape_sequence(self) -> String {
        format!("\u{1b}[{}m", self.code())
    }
}

impl fmt::Display for AnsiElement {
    /// Writes the ANSI code of this element.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.code())
    }
}

/// A [`PropertySource`] resolving ANSI escape codes by name.
///
/// The following properties are exposed:
///
/// - `AnsiStyle.<NAME>` and `Ansi.<NAME>` for styles,
/// - `AnsiColor.<NAME>` and `Ansi.<NAME>` for foreground colors,
/// - `AnsiBackground.<NAME>` and `Ansi.BG_<NAME>` for background colors,
/// - `AnsiColor.<0-255>` and `AnsiBackground.<0-255>` for 8-bit colors.
#[derive(Clone)]
pub struct AnsiPropertySource {
    /// The named elements, keyed by property name.
    base: BasePropertySource<HashMap<String, AnsiElement>>,
    /// Whether property values are returned as full escape sequences.
    ///
    /// When `false` the raw codes are returned instead, for example `"31"`.
    encode: bool,
}

impl AnsiPropertySource {
    /// Creates a property source exposing the ANSI escape codes.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the property source.
    /// * `encode` - Whether property values are returned as full escape
    ///   sequences rather than raw codes.
    ///
    /// # Returns
    ///
    /// A new [`AnsiPropertySource`].
    pub fn new(name: impl Into<String>, encode: bool) -> Self {
        let mut elements: HashMap<String, AnsiElement> = HashMap::new();

        for style in AnsiStyle::VALUES {
            let element = AnsiElement::Style(style);
            elements.insert(format!("AnsiStyle.{}", style.name()), element);
            elements.insert(format!("Ansi.{}", style.name()), element);
        }
        for color in AnsiColor::VALUES {
            let element = AnsiElement::Color(color);
            elements.insert(format!("AnsiColor.{}", color.name()), element);
            elements.insert(format!("Ansi.{}", color.name()), element);
        }
        for background in AnsiBackground::VALUES {
            let element = AnsiElement::Background(background);
            elements.insert(format!("AnsiBackground.{}", background.name()), element);
            elements.insert(format!("Ansi.BG_{}", background.name()), element);
        }

        Self {
            base: BasePropertySource::new(name, elements),
            encode,
        }
    }

    /// Returns the name of this property source.
    pub fn name(&self) -> &str {
        self.base.name()
    }

    /// Returns the value of the given property, if it is an ANSI escape code.
    pub fn element(&self, name: &str) -> Option<AnsiElement> {
        self.base
            .source()
            .get(name)
            .copied()
            .or_else(|| eight_bit_element(name))
    }

    /// Returns the value written for the given element.
    fn value(&self, element: AnsiElement) -> String {
        if self.encode {
            element.escape_sequence()
        } else {
            element.code()
        }
    }
}

impl PropertySource<HashMap<String, AnsiElement>> for AnsiPropertySource {
    fn name(&self) -> &str {
        self.base.name()
    }

    /// Returns the value of the given property, or `None` when the name is not
    /// an ANSI escape code.
    fn property(&self, name: &str) -> Option<String> {
        self.element(name).map(|element| self.value(element))
    }

    fn source(&self) -> &HashMap<String, AnsiElement> {
        self.base.source()
    }
}

impl fmt::Debug for AnsiPropertySource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AnsiPropertySource")
            .field("name", &self.base.name())
            .field("encode", &self.encode)
            .finish()
    }
}

/// Returns the 8-bit color element for a name such as `AnsiColor.208`.
///
/// # Arguments
///
/// * `name` - The property name.
///
/// # Returns
///
/// The 8-bit color element, or `None` when the name is not an 8-bit color.
fn eight_bit_element(name: &str) -> Option<AnsiElement> {
    let (prefix, code) = name.rsplit_once('.')?;
    let code: u8 = code.parse().ok()?;
    match prefix {
        "AnsiColor" => Some(AnsiElement::EightBitColor(code)),
        "AnsiBackground" => Some(AnsiElement::EightBitBackground(code)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns a property source encoding its values as escape sequences.
    fn encoded_source() -> AnsiPropertySource {
        AnsiPropertySource::new(ANSI_PROPERTY_SOURCE_NAME, true)
    }

    #[test]
    fn exposes_foreground_color_as_escape_sequence() {
        let source = encoded_source();
        assert_eq!(
            PropertySource::property(&source, "AnsiColor.RED"),
            Some("\u{1b}[31m".to_string())
        );
    }

    #[test]
    fn exposes_color_through_ansi_alias() {
        let source = encoded_source();
        assert_eq!(
            PropertySource::property(&source, "Ansi.BRIGHT_GREEN"),
            Some("\u{1b}[92m".to_string())
        );
    }

    #[test]
    fn exposes_background_through_bg_alias() {
        let source = encoded_source();
        assert_eq!(
            PropertySource::property(&source, "Ansi.BG_BLUE"),
            Some("\u{1b}[44m".to_string())
        );
        assert_eq!(
            PropertySource::property(&source, "AnsiBackground.BLUE"),
            Some("\u{1b}[44m".to_string())
        );
    }

    #[test]
    fn exposes_style() {
        let source = encoded_source();
        assert_eq!(
            PropertySource::property(&source, "AnsiStyle.BOLD"),
            Some("\u{1b}[1m".to_string())
        );
    }

    #[test]
    fn exposes_eight_bit_color() {
        let source = encoded_source();
        assert_eq!(
            PropertySource::property(&source, "AnsiColor.208"),
            Some("\u{1b}[38;5;208m".to_string())
        );
        assert_eq!(
            PropertySource::property(&source, "AnsiBackground.233"),
            Some("\u{1b}[48;5;233m".to_string())
        );
    }

    #[test]
    fn returns_raw_codes_when_not_encoded() {
        let source = AnsiPropertySource::new(ANSI_PROPERTY_SOURCE_NAME, false);
        assert_eq!(
            PropertySource::property(&source, "AnsiColor.RED"),
            Some("31".to_string())
        );
    }

    #[test]
    fn ignores_unknown_property() {
        let source = encoded_source();
        assert_eq!(PropertySource::property(&source, "AnsiColor.UNKNOWN"), None);
        assert_eq!(PropertySource::property(&source, "AnsiColor.256"), None);
        assert_eq!(PropertySource::property(&source, "application.title"), None);
    }

    #[test]
    fn contains_named_and_eight_bit_properties() {
        let source = encoded_source();
        assert!(source.contains_property("AnsiStyle.UNDERLINE"));
        assert!(source.contains_property("AnsiColor.1"));
        assert!(!source.contains_property("AnsiColor.NONE"));
    }

    #[test]
    fn element_display_writes_the_ansi_code() {
        assert_eq!(AnsiElement::Color(AnsiColor::Red).to_string(), "31");
        assert_eq!(
            AnsiElement::EightBitColor(208).to_string(),
            "38;5;208"
        );
    }
}
