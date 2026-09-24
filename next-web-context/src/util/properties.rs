//! Port of `java.util.Properties`.
//!
//! The type is a map from `String` keys to `String` values that knows how to
//! read the two textual representations the JDK supports:
//!
//! - the `.properties` format ([`Properties::load_from_bytes`] /
//!   [`Properties::load_from_str`]), and
//! - the XML format of `Properties.loadFromXML`
//!   ([`Properties::load_from_xml`]).
//!
//! Like the JDK type, a fresh instance keeps the insertion order of the entries
//! unless it is created with [`Properties::create_sorted_properties`]`(true)`,
//! in which case the keys are enumerated in alphabetical order.

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::io;

use indexmap::IndexMap;

/// A persistent set of properties.
///
/// # Examples
///
/// ```rust
/// use next_web_context::util::Properties;
///
/// let mut properties = Properties::new();
/// properties.load_from_str("greeting = Hello\n");
///
/// assert_eq!(properties.get_property("greeting"), Some("Hello"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Properties {
    entries: IndexMap<String, String>,
    sorted: bool,
}

impl Properties {
    /// Creates an empty instance that keeps the insertion order of its entries.
    ///
    /// Equivalent to `CollectionFactory.createSortedProperties(false)`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty instance whose keys are enumerated in the requested
    /// order.
    ///
    /// Equivalent to `CollectionFactory.createSortedProperties(sorted)`: when
    /// `sorted` is `false` the insertion order of the entries is kept, when it
    /// is `true` the keys are enumerated alphabetically.
    ///
    /// # Arguments
    ///
    /// * `sorted` - Whether the keys are enumerated alphabetically.
    pub fn create_sorted_properties(sorted: bool) -> Self {
        Self {
            entries: IndexMap::new(),
            sorted,
        }
    }

    /// Returns whether the keys are enumerated alphabetically.
    pub fn is_sorted(&self) -> bool {
        self.sorted
    }

    /// Returns the value of the given key, or `None` when there is none.
    ///
    /// Equivalent to `Properties.getProperty(String)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up.
    pub fn get_property(&self, key: &str) -> Option<&str> {
        self.get(key)
    }

    /// Returns the value of the given key, or `None` when there is none.
    ///
    /// Equivalent to `Properties.get(Object)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Returns the value of the given key, or the given default when there is
    /// none.
    ///
    /// Equivalent to `Properties.getOrDefault(Object, Object)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up.
    /// * `default` - The value returned when the key is not present.
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    /// Stores the given value under the given key.
    ///
    /// Equivalent to `Properties.setProperty(String, String)`. The value the
    /// key had before, if any, is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The key the value is stored under.
    /// * `value` - The value to store.
    pub fn set_property(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.entries.insert(key.into(), value.into())
    }

    /// Stores the given value under the given key.
    ///
    /// Equivalent to `Properties.put(Object, Object)`. The value the key had
    /// before, if any, is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The key the value is stored under.
    /// * `value` - The value to store.
    pub fn put(&mut self, key: impl Into<String>, value: impl Into<String>) -> Option<String> {
        self.set_property(key, value)
    }

    /// Copies every entry of the given instance into this one.
    ///
    /// Equivalent to `Properties.putAll(Map)`.
    ///
    /// # Arguments
    ///
    /// * `other` - The entries to copy. Entries whose key is already present
    ///   replace the current value, and keep the position of the current entry.
    pub fn put_all(&mut self, other: &Properties) {
        for (key, value) in other.entries.iter() {
            self.set_property(key.clone(), value.clone());
        }
    }

    /// Returns whether the given key is present.
    ///
    /// Equivalent to `Properties.containsKey(Object)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look for.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Removes the given key and returns the value it had.
    ///
    /// Equivalent to `Properties.remove(Object)`.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to remove.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.entries.shift_remove(key)
    }

    /// Removes every entry.
    ///
    /// Equivalent to `Properties.clear()`.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the number of entries.
    ///
    /// Equivalent to `Properties.size()`.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether there is no entry.
    ///
    /// Equivalent to `Properties.isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the keys in the enumeration order of this instance.
    ///
    /// Equivalent to `Properties.stringPropertyNames()`.
    pub fn keys(&self) -> Vec<&str> {
        if self.sorted {
            return self.sorted_keys();
        }

        self.entries.keys().map(String::as_str).collect()
    }

    /// Returns the values in the enumeration order of this instance.
    ///
    /// Equivalent to `Properties.values()`.
    pub fn values(&self) -> Vec<&str> {
        if self.sorted {
            return self
                .sorted_entries()
                .into_iter()
                .map(|(_, value)| value)
                .collect();
        }

        self.entries.values().map(String::as_str).collect()
    }

    /// Returns the entries in the enumeration order of this instance.
    ///
    /// Equivalent to `Properties.entrySet()`.
    pub fn iter(&self) -> Vec<(&str, &str)> {
        if self.sorted {
            return self.sorted_entries();
        }

        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect()
    }

    /// Returns the keys sorted alphabetically, whatever the enumeration order
    /// of this instance is.
    pub fn sorted_keys(&self) -> Vec<&str> {
        self.entries
            .keys()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(String::as_str)
            .collect()
    }

    /// Returns the entries sorted by key, whatever the enumeration order of
    /// this instance is.
    pub fn sorted_entries(&self) -> Vec<(&str, &str)> {
        let mut entries = self
            .entries
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        entries
    }

    /// Returns a copy of the entries as a [`HashMap`].
    pub fn to_map(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// Loads the `.properties` format of `Properties.load(InputStream)`.
    ///
    /// The bytes are decoded as UTF-8 and every entry is stored in this
    /// instance, replacing the value an already present key has.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw content of a `.properties` file.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the content is not valid UTF-8.
    pub fn load_from_bytes(&mut self, data: &[u8]) -> io::Result<()> {
        let content = decode(data)?;
        self.load_from_str(&content);
        Ok(())
    }

    /// Loads the `.properties` format of `Properties.load(Reader)`.
    ///
    /// Every entry is stored in this instance, replacing the value an already
    /// present key has.
    ///
    /// # Arguments
    ///
    /// * `content` - The decoded content of a `.properties` file.
    pub fn load_from_str(&mut self, content: &str) {
        let mut logical_line = String::new();

        for raw_line in content.split_inclusive(['\n', '\r']) {
            let raw_line = raw_line.trim_end_matches(['\n', '\r']);
            let continued = ends_with_unescaped_backslash(raw_line);
            let segment = if continued {
                &raw_line[..raw_line.len() - 1]
            } else {
                raw_line
            };

            if logical_line.is_empty() {
                logical_line.push_str(segment);
            } else {
                // Continuation lines ignore their leading whitespace, as
                // defined by the `.properties` syntax.
                logical_line.push_str(segment.trim_start());
            }

            if continued {
                continue;
            }

            self.insert_line(&logical_line);
            logical_line.clear();
        }

        if !logical_line.is_empty() {
            self.insert_line(&logical_line);
        }
    }

    /// Loads the XML format of `Properties.loadFromXML(InputStream)`.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw content of a `.xml` properties file.
    ///
    /// # Errors
    ///
    /// Returns an [`io::Error`] when the content is not valid UTF-8 or is not a
    /// well formed `<properties>` document.
    pub fn load_from_xml(&mut self, data: &[u8]) -> io::Result<()> {
        let content = decode(data)?;
        parse_xml(&content, self)
    }

    /// Stores a single logical line of a `.properties` file.
    fn insert_line(&mut self, line: &str) {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            return;
        }

        let (key, value) = split_property(trimmed);
        let key = unescape(key.trim());
        if key.is_empty() {
            return;
        }

        self.set_property(key, unescape(value.trim_start()));
    }
}

impl fmt::Display for Properties {
    /// Writes the entries the way `Properties.list(PrintStream)` does.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (key, value) in self.iter() {
            writeln!(f, "{key}={value}")?;
        }
        Ok(())
    }
}

impl<K, V> FromIterator<(K, V)> for Properties
where
    K: Into<String>,
    V: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        let mut properties = Self::new();
        properties.extend(iter);
        properties
    }
}

impl<K, V> Extend<(K, V)> for Properties
where
    K: Into<String>,
    V: Into<String>,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.set_property(key, value);
        }
    }
}

impl From<HashMap<String, String>> for Properties {
    fn from(map: HashMap<String, String>) -> Self {
        map.into_iter().collect()
    }
}

impl From<Properties> for HashMap<String, String> {
    fn from(properties: Properties) -> Self {
        properties.to_map()
    }
}

/// Decodes the raw content of a properties file.
///
/// The JDK decodes the bytes with the charset it is given, by default
/// ISO-8859-1; a Rust string is UTF-8, so the content is decoded as UTF-8 and
/// malformed input is reported instead of being silently replaced.
///
/// # Arguments
///
/// * `data` - The raw content of a properties file.
fn decode(data: &[u8]) -> io::Result<String> {
    std::str::from_utf8(data)
        .map(str::to_owned)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// Returns whether the line ends with an odd number of backslashes, which is
/// how the `.properties` syntax escapes a line break.
fn ends_with_unescaped_backslash(value: &str) -> bool {
    let mut count = 0;
    for byte in value.as_bytes().iter().rev() {
        if *byte == b'\\' {
            count += 1;
        } else {
            break;
        }
    }

    count % 2 == 1
}

/// Splits a logical line into its key and its value.
///
/// The key ends at the first unescaped `=`, `:` or whitespace; the optional
/// `=` or `:` and the whitespace around the separator are not part of the
/// value.
fn split_property(line: &str) -> (&str, &str) {
    let mut escaped = false;

    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '=' | ':' => return (&line[..index], &line[index + character.len_utf8()..]),
            character if character.is_whitespace() => {
                let mut value_start = index;

                while let Some(next) = line[value_start..].chars().next() {
                    if !next.is_whitespace() {
                        break;
                    }
                    value_start += next.len_utf8();
                }

                if let Some(next) = line[value_start..]
                    .chars()
                    .next()
                    .filter(|next| *next == '=' || *next == ':')
                {
                    value_start += next.len_utf8();

                    while let Some(space) = line[value_start..].chars().next() {
                        if !space.is_whitespace() {
                            break;
                        }
                        value_start += space.len_utf8();
                    }
                }

                return (&line[..index], &line[value_start..]);
            }
            _ => {}
        }
    }

    (line, "")
}

/// Resolves the escape sequences the `.properties` syntax defines.
fn unescape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }

        match characters.next() {
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('n') => result.push('\n'),
            Some('f') => result.push('\u{000C}'),
            Some('u') => {
                let remaining = characters.as_str().as_bytes();
                if remaining.len() >= 4
                    && remaining[..4].iter().all(u8::is_ascii_hexdigit)
                    && let Ok(code_point) = u32::from_str_radix(
                        std::str::from_utf8(&remaining[..4]).unwrap_or_default(),
                        16,
                    )
                    && let Some(decoded) = char::from_u32(code_point)
                {
                    result.push(decoded);
                    for _ in 0..4 {
                        characters.next();
                    }
                    continue;
                }

                // An invalid `\u` sequence is kept verbatim.
                result.push('\\');
                result.push('u');
            }
            // Every other escaped character stands for itself: `\,`, `\ ` and
            // `\:` are the escapes the `.properties` syntax defines for the
            // separators.
            Some(other) => result.push(other),
            None => result.push('\\'),
        }
    }

    result
}

/// Reads the `<properties>` document `Properties.loadFromXML` accepts.
fn parse_xml(content: &str, properties: &mut Properties) -> io::Result<()> {
    let mut cursor = 0;
    let mut seen_root = false;

    while let Some(offset) = content[cursor..].find('<') {
        let tag_start = cursor + offset;
        let tag_end = content[tag_start..]
            .find('>')
            .map(|offset| tag_start + offset)
            .ok_or_else(|| invalid_data("unterminated tag in properties XML"))?;
        let tag = &content[tag_start + 1..tag_end];

        if tag.starts_with("!--") {
            let end = content[tag_start..]
                .find("-->")
                .map(|offset| tag_start + offset + 3)
                .ok_or_else(|| invalid_data("unterminated comment in properties XML"))?;
            cursor = end;
            continue;
        }

        // Processing instructions, the document type declaration and the
        // closing tags carry no property.
        if tag.starts_with('?') || tag.starts_with('!') || tag.starts_with('/') {
            cursor = tag_end + 1;
            continue;
        }

        let (name, attributes) = match tag.split_once(char::is_whitespace) {
            Some((name, attributes)) => (name, attributes),
            None => (tag, ""),
        };
        let self_closing = name.ends_with('/') || attributes.trim_end().ends_with('/');
        let name = name.trim_end_matches('/');

        match name {
            "properties" => {
                seen_root = true;
                cursor = tag_end + 1;
            }
            "comment" => cursor = tag_end + 1,
            "entry" => {
                if !seen_root {
                    return Err(invalid_data("missing <properties> root element"));
                }

                let key = attribute(attributes, "key")
                    .ok_or_else(|| invalid_data("an <entry> element requires a key attribute"))?;
                let key = decode_entities(&key)?;

                if self_closing {
                    properties.set_property(key, "");
                    cursor = tag_end + 1;
                    continue;
                }

                let value_start = tag_end + 1;
                let value_end = content[value_start..]
                    .find("</")
                    .map(|offset| value_start + offset)
                    .ok_or_else(|| invalid_data("unterminated <entry> element"))?;
                let closing_end = content[value_end..]
                    .find('>')
                    .map(|offset| value_end + offset)
                    .ok_or_else(|| invalid_data("unterminated </entry> tag"))?;

                let value = decode_entities(&content[value_start..value_end])?;
                properties.set_property(key, value);
                cursor = closing_end + 1;
            }
            _ => {
                return Err(invalid_data(format!(
                    "unexpected element <{name}> in properties XML"
                )));
            }
        }
    }

    if !seen_root {
        return Err(invalid_data("missing <properties> root element"));
    }

    Ok(())
}

/// Returns the value of the given attribute of a start tag.
fn attribute(attributes: &str, name: &str) -> Option<String> {
    let mut rest = attributes.trim().trim_end_matches('/').trim();

    while !rest.is_empty() {
        let attribute_name = rest
            .split(['=', ' ', '\t', '\n', '\r'])
            .next()
            .unwrap_or_default();
        if attribute_name.is_empty() {
            break;
        }

        let after_name = rest[attribute_name.len()..].trim_start();
        let Some(after_equal) = after_name.strip_prefix('=').map(str::trim_start) else {
            rest = after_name;
            continue;
        };

        let quote = after_equal.chars().next()?;
        let (value, remaining) = if quote == '"' || quote == '\'' {
            let quoted = &after_equal[quote.len_utf8()..];
            let end = quoted.find(quote)?;
            (&quoted[..end], &quoted[end + quote.len_utf8()..])
        } else {
            let end = after_equal
                .find(char::is_whitespace)
                .unwrap_or(after_equal.len());
            (&after_equal[..end], &after_equal[end..])
        };

        if attribute_name == name {
            return Some(value.to_owned());
        }

        rest = remaining.trim_start();
    }

    None
}

/// Resolves the predefined and the numeric XML entities.
fn decode_entities(value: &str) -> io::Result<String> {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(index) = rest.find('&') {
        result.push_str(&rest[..index]);
        let entity_end = rest[index..]
            .find(';')
            .map(|offset| index + offset)
            .ok_or_else(|| invalid_data("unterminated XML entity"))?;
        let entity = &rest[index + 1..entity_end];

        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let invalid = || invalid_data(format!("invalid XML entity &{entity};"));

                let code_point = if let Some(digits) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(digits, 16).map_err(|_| invalid())?
                } else if let Some(digits) = entity.strip_prefix('#') {
                    digits.parse::<u32>().map_err(|_| invalid())?
                } else {
                    return Err(invalid_data(format!("unknown XML entity &{entity};")));
                };

                char::from_u32(code_point).ok_or_else(invalid)?
            }
        };

        result.push(character);

        rest = &rest[entity_end + 1..];
    }

    result.push_str(rest);
    Ok(result)
}

/// Builds the error returned for malformed properties XML.
fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_from_str_handles_continuations_comments_and_unicode() {
        let mut properties = Properties::new();
        properties.load_from_str(
            "# comment\n\
             ! comment\n\
             greeting = Hello\\\n\
                 World\n\
             escaped.key : value\\:\\=\\#\\!\n\
             unicode = \\u4F60\\u597D\n\
             separator value\n",
        );

        assert_eq!(properties.get_property("greeting"), Some("HelloWorld"));
        assert_eq!(properties.get_property("escaped.key"), Some("value:=#!"));
        assert_eq!(properties.get_property("unicode"), Some("\u{4F60}\u{597D}"));
        assert_eq!(properties.get_property("separator"), Some("value"));
    }

    #[test]
    fn load_from_bytes_reads_utf_8() {
        let mut properties = Properties::new();
        properties
            .load_from_bytes("greeting = caf\u{E9}\n".as_bytes())
            .unwrap();

        assert_eq!(properties.get_property("greeting"), Some("caf\u{E9}"));
    }

    #[test]
    fn load_from_bytes_rejects_input_that_is_not_utf_8() {
        let mut properties = Properties::new();
        let error = properties
            .load_from_bytes(b"greeting = caf\xE9\n")
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn load_from_xml_reads_entries_and_entities() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">
<properties>
  <comment>Common messages</comment>
  <entry key="greeting">Hello &amp; welcome</entry>
  <entry key="name">Your name is {0}</entry>
  <entry key="empty"></entry>
</properties>
"#;

        let mut properties = Properties::new();
        properties.load_from_xml(xml.as_bytes()).unwrap();

        assert_eq!(properties.get_property("greeting"), Some("Hello & welcome"));
        assert_eq!(properties.get_property("name"), Some("Your name is {0}"));
        assert_eq!(properties.get_property("empty"), Some(""));
    }

    #[test]
    fn load_from_xml_rejects_unknown_elements() {
        let mut properties = Properties::new();
        let error = properties
            .load_from_xml(b"<properties><unknown/></properties>")
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn extends_and_iterates_in_insertion_order() {
        let mut properties = Properties::new();
        properties.set_property("b", "2");
        properties.set_property("a", "1");
        properties.put_all(&Properties::from_iter([("c", "3")]));

        assert_eq!(properties.keys(), ["b", "a", "c"]);
        assert_eq!(properties.sorted_keys(), ["a", "b", "c"]);
        assert_eq!(properties.get("missing"), None);
        assert_eq!(properties.get_or("missing", "fallback"), "fallback");
        assert_eq!(properties.remove("a"), Some("1".to_string()));
        assert!(!properties.contains_key("a"));
    }

    #[test]
    fn sorted_properties_enumerate_keys_alphabetically() {
        let mut properties = Properties::create_sorted_properties(true);
        properties.set_property("b", "2");
        properties.set_property("a", "1");

        assert!(properties.is_sorted());
        assert_eq!(properties.keys(), ["a", "b"]);
        assert_eq!(properties.iter(), [("a", "1"), ("b", "2")]);
    }
}
