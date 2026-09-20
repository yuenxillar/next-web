use std::io::{self, BufRead, Read};

use next_web_core::{io::Resource, util::indexmap::IndexMap};

/// Loads `.properties` files into a list of [`Document`]s.
///
/// Also supports expansion of `name[]=a,b,c` list-style values.
///
/// Each logical document in the file becomes a [`Document`]. A new document is
/// signalled by a comment line consisting of `---` (three hyphens), matching
/// the multi-document convention.
///
/// All input is decoded as UTF-8.
pub struct PropertiesLoader<'a> {
    resource: &'a dyn Resource,
}

impl<'a> PropertiesLoader<'a> {
    /// Create a new loader for the given resource.
    pub fn new(resource: &'a dyn Resource) -> Self {
        Self { resource }
    }

    /// Load `.properties` data and return the parsed documents, expanding
    /// list-style values.
    pub fn load(&self) -> io::Result<Vec<Document>> {
        self.load_with_lists(true)
    }

    /// Load `.properties` data and return the parsed documents.
    ///
    /// `expand_lists` controls whether `name[]=a,b,c` shortcuts are expanded
    /// into indexed keys.
    pub fn load_with_lists(&self, expand_lists: bool) -> io::Result<Vec<Document>> {
        let mut documents: Vec<Document> = Vec::new();
        let mut document = Document::new();
        let mut buffer = String::new();

        let mut reader = CharacterReader::new(self.resource)?;

        loop {
            if !reader.read()? {
                break;
            }

            if reader.is_comment_prefix_character() {
                let comment_prefix_character = reader.character();
                if self.is_new_document(&mut reader)? {
                    if !document.is_empty() {
                        documents.push(document);
                    }
                    document = Document::new();
                } else {
                    if document.is_empty() && !documents.is_empty() {
                        document = documents.pop().unwrap();
                    }
                    reader.set_last_line_comment_prefix_character(Some(comment_prefix_character));
                    reader.skip_comment()?;
                }
            } else {
                reader.set_last_line_comment_prefix_character(None);
                self.load_key_and_value(expand_lists, &mut document, &mut reader, &mut buffer)?;
            }
        }

        if !document.is_empty() && !documents.iter().any(|d| d == &document) {
            documents.push(document);
        }

        Ok(documents)
    }

    /// Load a single key/value (or key/list) pair from the reader.
    fn load_key_and_value(
        &self,
        expand_lists: bool,
        document: &mut Document,
        reader: &mut CharacterReader,
        buffer: &mut String,
    ) -> io::Result<()> {
        let mut key = self.load_key(buffer, reader)?.trim().to_string();

        if expand_lists && key.ends_with("[]") {
            key.truncate(key.len() - 2);
            let mut index = 0usize;
            loop {
                let value = self.load_value(buffer, reader, true)?;
                document.put(format!("{}[{}]", key, index), value);
                index += 1;
                if !reader.is_end_of_line() {
                    reader.read()?;
                }
                if reader.is_end_of_line() {
                    break;
                }
            }
        } else {
            let value = self.load_value(buffer, reader, false)?;
            document.put(key, value);
        }

        Ok(())
    }

    /// Read a key up to the property delimiter (`=` or `:`) or end of line.
    fn load_key(&self, buffer: &mut String, reader: &mut CharacterReader) -> io::Result<String> {
        buffer.clear();
        let mut previous_whitespace = false;

        while !reader.is_end_of_line() {
            if reader.is_property_delimiter() {
                reader.read()?;
                return Ok(buffer.clone());
            }
            if !reader.is_white_space() && previous_whitespace {
                return Ok(buffer.clone());
            }
            previous_whitespace = reader.is_white_space();
            buffer.push(reader.character());
            reader.read()?;
        }

        Ok(buffer.clone())
    }

    /// Read a value, optionally splitting on list delimiters (commas).
    ///
    /// Returns the raw string value.
    fn load_value(
        &self,
        buffer: &mut String,
        reader: &mut CharacterReader,
        split_lists: bool,
    ) -> io::Result<String> {
        buffer.clear();

        while reader.is_white_space() && !reader.is_end_of_line() {
            reader.read()?;
        }

        while !reader.is_end_of_line() && !(split_lists && reader.is_list_delimiter()) {
            buffer.push(reader.character());
            reader.read()?;
        }

        Ok(buffer.clone())
    }

    /// Detect the start of a new document: a line consisting of `---`.
    fn is_new_document(&self, reader: &mut CharacterReader) -> io::Result<bool> {
        if reader.is_same_last_line_comment_prefix() {
            return Ok(false);
        }

        let mut result = reader.location().column <= 1;
        result = result && Self::read_and_expect(reader, CharacterReader::is_hyphen_character)?;
        result = result && Self::read_and_expect(reader, CharacterReader::is_hyphen_character)?;
        result = result && Self::read_and_expect(reader, CharacterReader::is_hyphen_character)?;

        if !reader.is_end_of_line() {
            reader.read()?;
            reader.skip_whitespace()?;
        }

        Ok(result && reader.is_end_of_line())
    }

    /// Advance the reader by one character and check the predicate.
    fn read_and_expect<F>(reader: &mut CharacterReader, check: F) -> io::Result<bool>
    where
        F: Fn(&CharacterReader) -> bool,
    {
        reader.read()?;
        Ok(check(reader))
    }
}

/// Reads characters from the source resource, skipping comments, handling
/// multi-line values, and tracking `\` escapes.
///
/// Input is decoded as UTF-8.
pub struct CharacterReader {
    reader: Box<dyn BufRead>,
    /// Current line number, starting at 1.
    line_number: usize,
    /// Current column number.
    column_number: usize,
    escaped: bool,
    character: Option<char>,
    last_line_comment_prefix_character: Option<char>,
}

impl CharacterReader {
    /// Maps escape letters to their replacement characters.
    const ESCAPES: [(char, char); 4] = [('t', '\t'), ('r', '\r'), ('n', '\n'), ('f', '\u{000C}')];

    /// Create a reader over the resource, decoding as UTF-8.
    pub fn new(resource: &dyn Resource) -> io::Result<Self> {
        let bytes = resource.get_content()?.into_owned();
        Ok(Self {
            reader: Box::new(io::Cursor::new(bytes)),
            line_number: 1,
            column_number: 0,
            escaped: false,
            character: None,
            last_line_comment_prefix_character: None,
        })
    }

    /// Advance to the next character, handling escapes and line/column
    /// tracking.
    ///
    /// Returns `false` at end of file.
    pub fn read(&mut self) -> io::Result<bool> {
        self.escaped = false;
        self.character = self.read_char()?;

        self.column_number += 1;

        if self.column_number == 1 {
            self.skip_whitespace()?;
        }

        match self.character {
            Some('\\') => {
                self.escaped = true;
                self.read_escaped()?;
            }
            Some('\n') => {
                self.line_number += 1;
                self.column_number = 0;
            }
            _ => {}
        }

        Ok(!self.is_end_of_file())
    }

    /// Read a single UTF-8 character from the underlying byte reader.
    fn read_char(&mut self) -> io::Result<Option<char>> {
        let mut buf = [0u8; 1];
        let n = self.reader.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }

        let byte = buf[0];
        if byte < 0x80 {
            return Ok(Some(byte as char));
        }

        let width = if byte >= 0xF0 {
            4
        } else if byte >= 0xE0 {
            3
        } else {
            2
        };

        let mut bytes = [0u8; 4];
        bytes[0] = byte;
        for i in 1..width {
            let mut b = [0u8; 1];
            let n = self.reader.read(&mut b)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "truncated UTF-8 sequence",
                ));
            }
            bytes[i] = b[0];
        }

        let s = std::str::from_utf8(&bytes[..width])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(s.chars().next())
    }

    fn skip_whitespace(&mut self) -> io::Result<()> {
        while self.is_white_space() {
            self.character = self.read_char()?;
            self.column_number += 1;
        }
        Ok(())
    }

    fn set_last_line_comment_prefix_character(&mut self, value: Option<char>) {
        self.last_line_comment_prefix_character = value;
    }

    fn skip_comment(&mut self) -> io::Result<()> {
        while self.character != Some('\n') && self.character.is_some() {
            self.character = self.read_char()?;
        }
        if self.character == Some('\n') {
            self.line_number += 1;
        }
        self.column_number = 0;
        Ok(())
    }

    fn read_escaped(&mut self) -> io::Result<()> {
        self.character = self.read_char()?;

        if let Some(c) = self.character {
            if let Some((_, replacement)) = Self::ESCAPES.iter().find(|(letter, _)| *letter == c) {
                self.character = Some(*replacement);
            } else if c == '\n' {
                self.line_number += 1;
                self.column_number = 0;
                self.read()?;
            } else if c == 'u' {
                self.read_unicode()?;
            }
        }

        Ok(())
    }

    fn read_unicode(&mut self) -> io::Result<()> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = self.read_char()?;
            let nibble = match digit {
                Some(c @ '0'..='9') => c as u32 - '0' as u32,
                Some(c @ 'a'..='f') => c as u32 - 'a' as u32 + 10,
                Some(c @ 'A'..='F') => c as u32 - 'A' as u32 + 10,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Malformed \\uxxxx encoding.",
                    ));
                }
            };
            value = (value << 4) + nibble;
        }

        self.character = char::from_u32(value);
        Ok(())
    }

    /// Whether the current character is horizontal whitespace.
    pub fn is_white_space(&self) -> bool {
        !self.escaped && matches!(self.character, Some(' ') | Some('\t') | Some('\u{000C}'))
    }

    /// Whether the reader has reached end of file.
    pub fn is_end_of_file(&self) -> bool {
        self.character.is_none()
    }

    /// Whether the current character ends a logical line (EOF or unescaped
    /// newline).
    pub fn is_end_of_line(&self) -> bool {
        self.character.is_none() || (!self.escaped && self.character == Some('\n'))
    }

    /// Whether the current character is an unescaped comma.
    pub fn is_list_delimiter(&self) -> bool {
        !self.escaped && self.character == Some(',')
    }

    /// Whether the current character is an unescaped `=` or `:`.
    pub fn is_property_delimiter(&self) -> bool {
        !self.escaped && matches!(self.character, Some('=') | Some(':'))
    }

    /// Return the current character.
    pub fn character(&self) -> char {
        self.character.unwrap_or('\0')
    }

    /// Return the current line/column location.
    pub fn location(&self) -> Location {
        Location {
            line: self.line_number,
            column: self.column_number,
        }
    }

    /// Whether the current character matches the comment prefix seen on the
    /// previous line.
    pub fn is_same_last_line_comment_prefix(&self) -> bool {
        self.last_line_comment_prefix_character == self.character
    }

    /// Whether the current character begins a comment (`#` or `!`).
    pub fn is_comment_prefix_character(&self) -> bool {
        matches!(self.character, Some('#') | Some('!'))
    }

    /// Whether the current character is a hyphen.
    pub fn is_hyphen_character(&self) -> bool {
        self.character == Some('-')
    }
}

/// A single document within a properties file.
///
/// Preserves insertion order of keys.
#[derive(Debug, Default, PartialEq)]
pub struct Document {
    values: Vec<(String, String)>,
}

impl Document {
    /// Create an empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a key/value pair, replacing any existing entry with the same
    /// key. Empty keys are ignored.
    pub fn put(&mut self, key: String, value: String) {
        if key.is_empty() {
            return;
        }

        if let Some(entry) = self.values.iter_mut().find(|(k, _)| *k == key) {
            entry.1 = value;
        } else {
            self.values.push((key, value));
        }
    }

    /// Whether the document has no entries.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Consume the document and return its entries as an insertion-ordered
    /// map.
    pub fn into_map(self) -> IndexMap<String, String> {
        IndexMap::from_iter(self.values)
    }
}

/// A line/column location within a text resource.
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}
