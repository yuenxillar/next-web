use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// MIME type entry containing both MIME type and file extension
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimeTypeEntry {
    mime_type: String,
    extension: String,
}

impl MimeTypeEntry {
    /// Creates a new MIME type entry
    pub fn new(mime_type: String, extension: String) -> Self {
        Self {
            mime_type,
            extension,
        }
    }

    /// Returns the MIME type (e.g., "text/html")
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    /// Returns the file extension (e.g., "html")
    pub fn extension(&self) -> &str {
        &self.extension
    }
}

impl std::fmt::Display for MimeTypeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MIME type: {}, extension: {}",
            self.mime_type, self.extension
        )
    }
}

/// MIME type file parser that reads and parses mime.types configuration files
///
/// This parser supports both traditional format:
/// ```text
/// text/html               html htm HTML HTM
/// image/jpeg              jpeg jpg jpe JPG
/// ```
///
/// And extended format with name=value pairs:
/// ```text
/// type=text/html exts=html,htm,HTML,HTM
/// type=image/jpeg exts=jpeg,jpg,jpe,JPG
/// ```
///
/// The parser also handles line continuation with backslash at the end of line.
#[derive(Debug, Clone)]
pub struct MimeTypeFile {
    filename: Option<PathBuf>,
    type_hash: HashMap<String, MimeTypeEntry>,
}

impl MimeTypeFile {
    /// Creates an empty MIME type file (no mappings)
    pub fn new() -> Self {
        Self {
            filename: None,
            type_hash: HashMap::new(),
        }
    }

    /// Creates a MIME type file by loading from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path.as_ref())?;
        let reader = BufReader::new(file);
        let mut parser = Self {
            filename: Some(path.as_ref().to_path_buf()),
            type_hash: HashMap::new(),
        };
        parser.parse(reader)?;
        Ok(parser)
    }

    /// Creates a MIME type file by loading from an input stream
    pub fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;

        // Match Java's InputStreamReader(..., "iso-8859-1") behavior.
        let content: String = bytes.into_iter().map(char::from).collect();
        Self::from_str(&content)
    }

    /// Creates a MIME type file from a string containing configuration
    pub fn from_str(content: &str) -> io::Result<Self> {
        let reader = BufReader::new(content.as_bytes());
        let mut parser = Self {
            filename: None,
            type_hash: HashMap::new(),
        };
        parser.parse(reader)?;
        Ok(parser)
    }

    /// Returns the MIME type entry for the given file extension
    pub fn get_mime_type_entry(&self, file_ext: &str) -> Option<&MimeTypeEntry> {
        self.type_hash.get(file_ext)
    }

    /// Returns the MIME type string for the given file extension
    pub fn get_mime_type_string(&self, file_ext: &str) -> Option<&str> {
        self.get_mime_type_entry(file_ext)
            .map(|entry| entry.mime_type())
    }

    /// Appends additional MIME type definitions to the registry
    pub fn append_to_registry(&mut self, mime_types: &str) -> io::Result<()> {
        let reader = BufReader::new(mime_types.as_bytes());
        self.parse(reader)
    }

    /// Adds a custom MIME type mapping
    pub fn add_mapping(&mut self, mime_type: &str, extension: &str) {
        let entry = MimeTypeEntry::new(mime_type.to_string(), extension.to_string());
        self.type_hash.insert(extension.to_string(), entry);
    }

    /// Gets all registered extensions
    pub fn extensions(&self) -> Vec<&str> {
        self.type_hash.keys().map(|s| s.as_str()).collect()
    }

    /// Gets the number of registered mappings
    pub fn len(&self) -> usize {
        self.type_hash.len()
    }

    /// Returns true if no mappings are registered
    pub fn is_empty(&self) -> bool {
        self.type_hash.is_empty()
    }

    /// Parses the MIME type file content
    fn parse<R: BufRead>(&mut self, mut reader: R) -> io::Result<()> {
        let mut line_buffer = String::new();
        let mut prev_line: Option<String> = None;

        loop {
            line_buffer.clear();
            let bytes_read = reader.read_line(&mut line_buffer)?;

            if bytes_read == 0 {
                break;
            }

            let line = strip_line_terminator(&line_buffer);

            if let Some(prev) = prev_line.as_mut() {
                prev.push_str(line);
            } else {
                prev_line = Some(line.to_string());
            }

            let should_continue = prev_line
                .as_ref()
                .map(|entry| entry.ends_with('\\'))
                .unwrap_or(false);

            if should_continue {
                if let Some(prev) = prev_line.as_mut() {
                    prev.pop();
                }
            } else if let Some(entry) = prev_line.take() {
                self.parse_entry(&entry)?;
            }
        }

        if let Some(entry) = prev_line {
            self.parse_entry(&entry)?;
        }

        Ok(())
    }

    /// Parses a single entry line
    fn parse_entry(&mut self, line: &str) -> io::Result<()> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        // Check if this is extended format (contains '=')
        if matches!(line.find('='), Some(index) if index > 0) {
            self.parse_extended_format(line)?;
        } else {
            self.parse_traditional_format(line)?;
        }

        Ok(())
    }

    /// Parses traditional format: "mime_type ext1 ext2 ext3 ..."
    fn parse_traditional_format(&mut self, line: &str) -> io::Result<()> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(());
        }

        let mime_type = parts[0];

        // Parse all extensions
        for &ext in &parts[1..] {
            let entry = MimeTypeEntry::new(mime_type.to_string(), ext.to_string());

            if log_support::is_loggable() {
                log_support::log(&format!("Added: {}", entry));
            }

            self.type_hash.insert(ext.to_string(), entry);
        }

        Ok(())
    }

    /// Parses extended format: "type=text/html exts=html,htm,HTML,HTM"
    fn parse_extended_format(&mut self, line: &str) -> io::Result<()> {
        let mut mime_type: Option<String> = None;
        let mut tokenizer = LineTokenizer::new(line);

        while let Some(name) = tokenizer.next_token() {
            let value = if tokenizer.has_more_tokens()
                && tokenizer.next_token().as_deref() == Some("=")
                && tokenizer.has_more_tokens()
            {
                tokenizer.next_token()
            } else {
                None
            };

            let Some(value) = value else {
                if log_support::is_loggable() {
                    log_support::log(&format!("Bad .mime.types entry: {}", line));
                }
                return Ok(());
            };

            match name.as_str() {
                "type" => {
                    mime_type = Some(value);
                }
                "exts" => {
                    let Some(mime_type) = mime_type.as_ref() else {
                        if log_support::is_loggable() {
                            log_support::log(&format!("Bad .mime.types entry: {}", line));
                        }
                        return Ok(());
                    };

                    for ext in value.split(',').filter(|ext| !ext.is_empty()) {
                        let entry = MimeTypeEntry::new(mime_type.clone(), ext.to_string());

                        if log_support::is_loggable() {
                            log_support::log(&format!("Added: {}", entry));
                        }

                        self.type_hash.insert(ext.to_string(), entry);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl Default for MimeTypeFile {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for MimeTypeFile {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MimeTypeFile::from_str(s)
    }
}

/// Simple logging support (matching Java's LogSupport)
mod log_support {
    use std::sync::OnceLock;

    static LOGGING_ENABLED: OnceLock<bool> = OnceLock::new();

    /// Check if logging is enabled
    pub fn is_loggable() -> bool {
        *LOGGING_ENABLED.get_or_init(|| {
            std::env::var("JAF_LOGGING")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false)
        })
    }

    /// Log a message if logging is enabled
    pub fn log(message: &str) {
        if is_loggable() {
            eprintln!("[MimeTypeFile] {}", message);
        }
    }

    /// Enable logging programmatically
    pub fn enable_logging() {
        let _ = LOGGING_ENABLED.set(true);
    }
}

/// Line tokenizer for parsing extended format
struct LineTokenizer {
    chars: Vec<char>,
    position: usize,
}

impl LineTokenizer {
    fn new(line: &str) -> Self {
        Self {
            chars: line.chars().collect(),
            position: 0,
        }
    }

    fn has_more_tokens(&self) -> bool {
        let mut position = self.position;
        while position < self.chars.len() && self.chars[position].is_whitespace() {
            position += 1;
        }
        position < self.chars.len()
    }

    fn next_token(&mut self) -> Option<String> {
        // Skip whitespace
        while self.position < self.chars.len() && self.chars[self.position].is_whitespace() {
            self.position += 1;
        }

        if self.position >= self.chars.len() {
            return None;
        }

        if self.chars[self.position] == '=' {
            self.position += 1;
            return Some("=".to_string());
        }

        // Check for quoted string
        if self.chars[self.position] == '"' {
            self.position += 1; // Skip opening quote
            let start = self.position;
            while self.position < self.chars.len() && self.chars[self.position] != '"' {
                self.position += 1;
            }
            let token: String = self.chars[start..self.position].iter().collect();
            if self.position < self.chars.len() {
                self.position += 1; // Skip closing quote
            }
            return Some(token);
        }

        let start = self.position;

        // Regular token
        while self.position < self.chars.len()
            && !self.chars[self.position].is_whitespace()
            && self.chars[self.position] != '='
        {
            self.position += 1;
        }

        let token: String = self.chars[start..self.position].iter().collect();
        Some(token)
    }
}

fn strip_line_terminator(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_traditional_format() {
        let content = r#"
# Comment line
text/html               html htm HTML HTM
image/jpeg              jpeg jpg jpe JPG
application/pdf         pdf
"#;

        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("HTML"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("jpg"), Some("image/jpeg"));
        assert_eq!(mime_file.get_mime_type_string("jpeg"), Some("image/jpeg"));
        assert_eq!(
            mime_file.get_mime_type_string("pdf"),
            Some("application/pdf")
        );
        assert_eq!(mime_file.get_mime_type_string("unknown"), None);
    }

    #[test]
    fn test_extended_format() {
        let content = r#"
type=text/html exts=html,htm,HTML,HTM
type=image/jpeg exts=jpeg,jpg,jpe,JPG
type=application/pdf exts=pdf
"#;

        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("jpg"), Some("image/jpeg"));
        assert_eq!(
            mime_file.get_mime_type_string("pdf"),
            Some("application/pdf")
        );
    }

    #[test]
    fn test_line_continuation() {
        let content = r#"
text/html html \
          htm \
          HTML
image/jpeg jpeg jpg \
          jpe
"#;

        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("HTML"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("jpg"), Some("image/jpeg"));
        assert_eq!(mime_file.get_mime_type_string("jpe"), Some("image/jpeg"));
    }

    #[test]
    fn test_empty_and_comments() {
        let content = r#"
# This is a comment

# Another comment
text/plain txt text

"#;

        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("txt"), Some("text/plain"));
        assert_eq!(mime_file.get_mime_type_string("text"), Some("text/plain"));
        assert_eq!(mime_file.len(), 2);
    }

    #[test]
    fn test_append_to_registry() {
        let mut mime_file = MimeTypeFile::new();

        mime_file.append_to_registry("text/html html htm").unwrap();
        mime_file.append_to_registry("image/png png").unwrap();

        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("png"), Some("image/png"));
        assert_eq!(mime_file.len(), 3);
    }

    #[test]
    fn test_add_mapping() {
        let mut mime_file = MimeTypeFile::new();

        mime_file.add_mapping("application/json", "json");
        mime_file.add_mapping("text/xml", "xml");

        assert_eq!(
            mime_file.get_mime_type_string("json"),
            Some("application/json")
        );
        assert_eq!(mime_file.get_mime_type_string("xml"), Some("text/xml"));
        assert_eq!(mime_file.len(), 2);
    }

    #[test]
    fn test_get_entry() {
        let content = "text/plain txt";
        let mime_file = MimeTypeFile::from_str(content).unwrap();

        let entry = mime_file.get_mime_type_entry("txt").unwrap();
        assert_eq!(entry.mime_type(), "text/plain");
        assert_eq!(entry.extension(), "txt");

        let entry_str = entry.to_string();
        assert!(entry_str.contains("text/plain"));
        assert!(entry_str.contains("txt"));
    }

    #[test]
    fn test_case_insensitivity() {
        let content = "text/html HTML htm";
        let mime_file = MimeTypeFile::from_str(content).unwrap();

        // Java behavior preserves the parsed extension keys.
        assert_eq!(mime_file.get_mime_type_string("html"), None);
        assert_eq!(mime_file.get_mime_type_string("htm"), Some("text/html"));
        assert_eq!(mime_file.get_mime_type_string("HTML"), Some("text/html"));
    }

    #[test]
    fn test_line_tokenizer() {
        let mut tokenizer = LineTokenizer::new("type=text/html exts=html,htm");

        assert_eq!(tokenizer.next_token(), Some("type".to_string()));
        assert_eq!(tokenizer.next_token(), Some("=".to_string()));
        assert_eq!(tokenizer.next_token(), Some("text/html".to_string()));
        assert_eq!(tokenizer.next_token(), Some("exts".to_string()));
        assert_eq!(tokenizer.next_token(), Some("=".to_string()));
        assert_eq!(tokenizer.next_token(), Some("html,htm".to_string()));
        assert_eq!(tokenizer.next_token(), None);
    }

    #[test]
    fn test_empty_file() {
        let mime_file = MimeTypeFile::new();
        assert!(mime_file.is_empty());
        assert_eq!(mime_file.len(), 0);
        assert_eq!(mime_file.extensions().len(), 0);
    }

    #[test]
    fn test_from_file() -> io::Result<()> {
        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_mime.types");

        std::fs::write(&temp_file, "text/plain txt text\nimage/png png")?;

        let mime_file = MimeTypeFile::from_file(&temp_file)?;

        assert_eq!(mime_file.get_mime_type_string("txt"), Some("text/plain"));
        assert_eq!(mime_file.get_mime_type_string("text"), Some("text/plain"));
        assert_eq!(mime_file.get_mime_type_string("png"), Some("image/png"));

        // Clean up
        std::fs::remove_file(temp_file)?;

        Ok(())
    }

    #[test]
    fn test_quoted_strings() {
        let content = r#"type="text/html" exts="html,htm,HTML,HTM""#;
        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
    }

    #[test]
    fn test_extended_format_with_spaces_around_equals() {
        let content = "type = text/plain exts = txt,text";
        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("txt"), Some("text/plain"));
        assert_eq!(mime_file.get_mime_type_string("text"), Some("text/plain"));
    }

    #[test]
    fn test_continuation_requires_backslash_at_real_end() {
        let content = "text/plain txt\\  \ntext/html html\n";
        let mime_file = MimeTypeFile::from_str(content).unwrap();

        assert_eq!(mime_file.get_mime_type_string("txt\\"), Some("text/plain"));
        assert_eq!(mime_file.get_mime_type_string("html"), Some("text/html"));
    }
}
