use crate::core::usermodel::{font::Font, rich_text_string::RichTextString};

pub struct CsvRichTextString {
    /// The underlying string
    string: String,
}

impl CsvRichTextString {
    /// Creates a new CSV rich text string
    pub fn new<S: Into<String>>(string: S) -> Self {
        CsvRichTextString {
            string: string.into(),
        }
    }

    /// Gets a reference to the underlying string
    pub fn as_str(&self) -> &str {
        &self.string
    }

    /// Consumes the rich text string and returns the underlying String
    pub fn into_string(self) -> String {
        self.string
    }
}

impl RichTextString for CsvRichTextString {
    /// Apply font to a specific range (CSV doesn't support this)
    fn apply_font_range_with_index(
        &mut self,
        _start_index: u32,
        _end_index: u32,
        _font_index: u16,
    ) {
        // CSV doesn't support rich text formatting
    }

    /// Apply font to a specific range (CSV doesn't support this)
    fn apply_font_range(&mut self, _start_index: u32, _end_index: u32, _font: &dyn Font) {
        // CSV doesn't support rich text formatting
    }

    /// Apply font to the entire string (CSV doesn't support this)
    fn apply_font(&mut self, _font: &dyn Font) {
        // CSV doesn't support rich text formatting
    }

    /// Clear all formatting (CSV doesn't support this)
    fn clear_formatting(&mut self) {
        // CSV doesn't support rich text formatting
    }

    /// Get the string value
    fn get_string(&self) -> &str {
        &self.string
    }

    /// Get the length of the string
    fn length(&self) -> usize {
        self.string.len()
    }

    /// Get the number of formatting runs (always 0 for CSV)
    fn num_formatting_runs(&self) -> usize {
        0
    }

    /// Get the start index of a formatting run (always 0 for CSV)
    fn get_index_of_formatting_run(&self, _index: u32) -> usize {
        0
    }

    /// Apply font using font index (CSV doesn't support this)
    fn apply_font_with_index(&mut self, _font_index: u16) {
        // CSV doesn't support rich text formatting
    }
}

impl std::fmt::Display for CsvRichTextString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

impl From<String> for CsvRichTextString {
    fn from(string: String) -> Self {
        CsvRichTextString::new(string)
    }
}

impl From<&str> for CsvRichTextString {
    fn from(string: &str) -> Self {
        CsvRichTextString::new(string.to_string())
    }
}

impl Clone for CsvRichTextString {
    fn clone(&self) -> Self {
        CsvRichTextString {
            string: self.string.clone(),
        }
    }
}

impl PartialEq for CsvRichTextString {
    fn eq(&self, other: &Self) -> bool {
        self.string == other.string
    }
}

impl Eq for CsvRichTextString {}

impl std::hash::Hash for CsvRichTextString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.string.hash(state);
    }
}
