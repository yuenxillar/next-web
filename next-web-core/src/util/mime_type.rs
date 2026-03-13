use std::cmp::Ordering;
use std::collections::{HashMap, LinkedList};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::ops::Index;
use std::str::FromStr;

/// Represents a MIME type, consisting of a type and a subtype.
///
/// MIME types are used to identify the type of data being transmitted.
/// Examples include "text/plain", "application/json", etc.
#[derive(Clone)]
pub struct MimeType {
    type_: String,
    subtype: String,
    parameters: LinkedCaseInsensitiveMap<String>,
    resolved_charset: Option<String>, // Store charset name instead of Charset object
    to_string_value: Option<String>,
}

impl MimeType {
    /// Wildcard type constant.
    pub const WILDCARD_TYPE: &'static str = "*";

    /// Charset parameter name.
    const PARAM_CHARSET: &'static str = "charset";

    /// Create a new MimeType with the given type and wildcard subtype.
    pub fn new(type_: impl Into<String>) -> Self {
        Self::with_subtype(type_, Self::WILDCARD_TYPE)
    }

    /// Create a new MimeType with the given type and subtype.
    pub fn with_subtype(type_: impl Into<String>, subtype: impl Into<String>) -> Self {
        Self::with_parameters(type_, subtype, HashMap::new())
    }

    /// Create a new MimeType with the given type, subtype, and charset.
    pub fn with_charset(
        type_: impl Into<String>,
        subtype: impl Into<String>,
        charset: impl Into<String>,
    ) -> Self {
        let mut params = HashMap::new();
        params.insert(Self::PARAM_CHARSET.to_string(), charset.into());
        Self::with_parameters(type_, subtype, params)
    }

    /// Create a new MimeType with the given type, subtype, and parameters.
    pub fn with_parameters(
        type_: impl Into<String>,
        subtype: impl Into<String>,
        parameters: HashMap<String, String>,
    ) -> Self {
        let type_ = type_.into();
        let subtype = subtype.into();

        Self::check_token(&type_);
        Self::check_token(&subtype);

        let type_ = type_.to_lowercase();
        let subtype = subtype.to_lowercase();

        let mut resolved_charset = None;
        let mut param_map = LinkedCaseInsensitiveMap::new();

        if !parameters.is_empty() {
            for (key, value) in parameters {
                Self::check_parameters(&key, &value, &mut resolved_charset);
                param_map.insert(key, value);
            }
        }

        MimeType {
            type_,
            subtype,
            parameters: param_map,
            resolved_charset,
            to_string_value: None,
        }
    }

    /// Create a new MimeType from an existing one with a different charset.
    pub fn with_charset_from(other: &MimeType, charset: impl Into<String>) -> Self {
        let charset = charset.into();
        let mut params = other.parameters.clone();
        params.insert(Self::PARAM_CHARSET.to_string(), charset.clone());

        MimeType {
            type_: other.type_.clone(),
            subtype: other.subtype.clone(),
            parameters: params,
            resolved_charset: Some(charset),
            to_string_value: None,
        }
    }

    /// Create a new MimeType from an existing one with different parameters.
    pub fn with_parameters_from(other: &MimeType, parameters: HashMap<String, String>) -> Self {
        let mut resolved_charset = None;
        let mut param_map = LinkedCaseInsensitiveMap::new();

        for (key, value) in parameters {
            Self::check_parameters(&key, &value, &mut resolved_charset);
            param_map.insert(key, value);
        }

        MimeType {
            type_: other.type_.clone(),
            subtype: other.subtype.clone(),
            parameters: param_map,
            resolved_charset,
            to_string_value: None,
        }
    }

    /// Check if a token contains only valid characters.
    fn check_token(token: &str) {
        for ch in token.chars() {
            if !Self::is_token_char(ch) {
                panic!("Invalid token character '{}' in token \"{}\"", ch, token);
            }
        }
    }

    /// Check if parameters are valid.
    fn check_parameters(parameter: &str, value: &str, resolved_charset: &mut Option<String>) {
        assert!(!parameter.is_empty(), "'parameter' must not be empty");
        assert!(!value.is_empty(), "'value' must not be empty");

        Self::check_token(parameter);

        if parameter == Self::PARAM_CHARSET {
            if resolved_charset.is_none() {
                *resolved_charset = Some(Self::unquote(value).to_string());
            }
        } else if !Self::is_quoted_string(value) {
            Self::check_token(value);
        }
    }

    /// Check if a character is valid in a token.
    fn is_token_char(ch: char) -> bool {
        // RFC 2045: token = 1*<any CHAR except CTLs or separators>
        // CTLs = 0-31, 127
        // separators = "()<>@,;:\\\"/[]?={} \t"

        let c = ch as u32;
        if c > 127 {
            return true; // Non-ASCII allowed in tokens
        }

        // Check for CTLs
        if c <= 31 || c == 127 {
            return false;
        }

        // Check for separators
        match ch {
            '(' | ')' | '<' | '>' | '@' | ',' | ';' | ':' | '\\' | '"' | '/' | '[' | ']' | '?'
            | '=' | '{' | '}' | ' ' | '\t' => false,
            _ => true,
        }
    }

    /// Check if a string is quoted.
    fn is_quoted_string(s: &str) -> bool {
        if s.len() < 2 {
            return false;
        }
        (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\''))
    }

    /// Remove quotes from a quoted string.
    fn unquote(s: &str) -> &str {
        if Self::is_quoted_string(s) {
            &s[1..s.len() - 1]
        } else {
            s
        }
    }

    /// Check if this is a wildcard type.
    pub fn is_wildcard_type(&self) -> bool {
        self.type_ == Self::WILDCARD_TYPE
    }

    /// Check if this is a wildcard subtype.
    pub fn is_wildcard_subtype(&self) -> bool {
        self.subtype == Self::WILDCARD_TYPE || self.subtype.starts_with("*+")
    }

    /// Check if this is a concrete MIME type (not wildcard).
    pub fn is_concrete(&self) -> bool {
        !self.is_wildcard_type() && !self.is_wildcard_subtype()
    }

    /// Get the type.
    pub fn get_type(&self) -> &str {
        &self.type_
    }

    /// Get the subtype.
    pub fn get_subtype(&self) -> &str {
        &self.subtype
    }

    /// Get the subtype suffix (part after '+').
    pub fn get_subtype_suffix(&self) -> Option<&str> {
        self.subtype.rfind('+').map(|idx| &self.subtype[idx + 1..])
    }

    /// Get the charset parameter.
    pub fn get_charset(&self) -> Option<&str> {
        self.resolved_charset.as_deref()
    }

    /// Get a parameter value.
    pub fn get_parameter(&self, name: &str) -> Option<&String> {
        self.parameters.get(name)
    }

    /// Get all parameters.
    pub fn get_parameters(&self) -> &LinkedCaseInsensitiveMap<String> {
        &self.parameters
    }

    /// Check if this MIME type includes another.
    pub fn includes(&self, other: &MimeType) -> bool {
        if self.is_wildcard_type() {
            return true;
        }

        if self.type_ == other.type_ {
            if self.subtype == other.subtype {
                return true;
            }

            if self.is_wildcard_subtype() {
                let this_plus_idx = self.subtype.rfind('+');
                match this_plus_idx {
                    None => return true,
                    Some(idx) => {
                        let other_plus_idx = other.subtype.rfind('+');
                        if let Some(other_idx) = other_plus_idx {
                            let this_subtype_no_suffix = &self.subtype[..idx];
                            let this_subtype_suffix = &self.subtype[idx + 1..];
                            let other_subtype_suffix = &other.subtype[other_idx + 1..];

                            if this_subtype_suffix == other_subtype_suffix
                                && this_subtype_no_suffix == "*"
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if this MIME type is compatible with another.
    pub fn is_compatible_with(&self, other: Option<&MimeType>) -> bool {
        let other = match other {
            Some(o) => o,
            None => return false,
        };

        if self.is_wildcard_type() || other.is_wildcard_type() {
            return true;
        }

        if self.type_ == other.type_ {
            if self.subtype == other.subtype {
                return true;
            }

            if self.is_wildcard_subtype() || other.is_wildcard_subtype() {
                let this_suffix = self.get_subtype_suffix();
                let other_suffix = other.get_subtype_suffix();

                if self.subtype == "*" || other.subtype == "*" {
                    return true;
                }

                if self.is_wildcard_subtype() && this_suffix.is_some() {
                    return this_suffix.unwrap() == other.subtype || this_suffix == other_suffix;
                }

                if other.is_wildcard_subtype() && other_suffix.is_some() {
                    return other.subtype == other_suffix.unwrap() || other_suffix == this_suffix;
                }
            }
        }

        false
    }

    /// Check if type and subtype equal another MIME type (ignoring parameters).
    pub fn equals_type_and_subtype(&self, other: Option<&MimeType>) -> bool {
        match other {
            Some(o) => {
                self.type_.eq_ignore_ascii_case(&o.type_)
                    && self.subtype.eq_ignore_ascii_case(&o.subtype)
            }
            None => false,
        }
    }

    /// Check if this MIME type is present in a collection.
    pub fn is_present_in(&self, mime_types: &[MimeType]) -> bool {
        mime_types
            .iter()
            .any(|mt| self.equals_type_and_subtype(Some(mt)))
    }

    /// Check if parameters are equal to another MIME type.
    fn parameters_are_equal(&self, other: &MimeType) -> bool {
        if self.parameters.len() != other.parameters.len() {
            return false;
        }

        for (key, value) in self.parameters.iter() {
            let other_value = match other.parameters.get(key) {
                Some(v) => v,
                None => return false,
            };

            if key == Self::PARAM_CHARSET {
                if self.get_charset() != other.get_charset() {
                    return false;
                }
            } else if value != other_value {
                return false;
            }
        }

        true
    }

    /// Append parameters to a string builder.
    fn append_parameters(&self, builder: &mut String) {
        for (key, value) in self.parameters.iter() {
            builder.push(';');
            builder.push_str(key);
            builder.push('=');
            builder.push_str(value);
        }
    }

    /// Parse a MIME type from a string.
    pub fn parse(value: &str) -> Result<MimeType, String> {
        MimeTypeUtils::parse_mime_type(value)
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = &self.to_string_value {
            return write!(f, "{}", s);
        }

        let mut builder = String::new();
        builder.push_str(&self.type_);
        builder.push('/');
        builder.push_str(&self.subtype);
        self.append_parameters(&mut builder);

        // Cache the value
        let s = builder;

        // This is a bit of a hack since we can't mutate in fmt
        // We'll just write and not cache
        write!(f, "{}", s)?;

        // In a real implementation, you'd want to cache this
        // For now, we'll just return
        Ok(())
    }
}

impl fmt::Debug for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MimeType")
            .field("type", &self.type_)
            .field("subtype", &self.subtype)
            .field("parameters", &self.parameters)
            .field("charset", &self.resolved_charset)
            .finish()
    }
}

impl PartialEq for MimeType {
    fn eq(&self, other: &Self) -> bool {
        self.type_.eq_ignore_ascii_case(&other.type_)
            && self.subtype.eq_ignore_ascii_case(&other.subtype)
            && self.parameters_are_equal(other)
    }
}

impl Eq for MimeType {}

impl Hash for MimeType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_.to_lowercase().hash(state);
        self.subtype.to_lowercase().hash(state);

        // Sort parameters for consistent hashing
        let mut keys: Vec<&String> = self.parameters.keys().collect();
        keys.sort();

        for key in keys {
            key.to_lowercase().hash(state);
            if let Some(value) = self.parameters.get(key) {
                value.hash(state);
            }
        }
    }
}

impl PartialOrd for MimeType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MimeType {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare type
        let type_cmp = self.type_.cmp(&other.type_);
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        // Compare subtype
        let subtype_cmp = self.subtype.cmp(&other.subtype);
        if subtype_cmp != Ordering::Equal {
            return subtype_cmp;
        }

        // Compare parameter count (more parameters = more specific)
        let param_len_cmp = self.parameters.len().cmp(&other.parameters.len());
        if param_len_cmp != Ordering::Equal {
            return param_len_cmp.reverse(); // Reverse so more parameters come first
        }

        // Compare parameters lexicographically
        let mut this_keys: Vec<&String> = self.parameters.keys().collect();
        let mut other_keys: Vec<&String> = other.parameters.keys().collect();

        this_keys.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        other_keys.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));

        for (this_key, other_key) in this_keys.iter().zip(other_keys.iter()) {
            let key_cmp = this_key.to_lowercase().cmp(&other_key.to_lowercase());
            if key_cmp != Ordering::Equal {
                return key_cmp;
            }

            if *this_key == Self::PARAM_CHARSET {
                let this_charset = self.get_charset().unwrap_or("");
                let other_charset = other.get_charset().unwrap_or("");
                let charset_cmp = this_charset.cmp(other_charset);
                if charset_cmp != Ordering::Equal {
                    return charset_cmp;
                }
            } else {
                let this_value = self.parameters.get(*this_key).unwrap();
                let other_value = other.parameters.get(*other_key).unwrap();
                let value_cmp = this_value.cmp(other_value);
                if value_cmp != Ordering::Equal {
                    return value_cmp;
                }
            }
        }

        Ordering::Equal
    }
}

impl FromStr for MimeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MimeType::parse(s)
    }
}

/// Utility class for parsing and comparing MIME types.
pub struct MimeTypeUtils;

impl MimeTypeUtils {
    /// Parse a MIME type from a string.
    pub fn parse_mime_type(value: &str) -> Result<MimeType, String> {
        // Simple parser - in a real implementation, this would be more robust
        let parts: Vec<&str> = value.split(';').collect();
        if parts.is_empty() {
            return Err("Invalid MIME type".to_string());
        }

        let type_parts: Vec<&str> = parts[0].split('/').collect();
        if type_parts.len() != 2 {
            return Err("MIME type must have type and subtype separated by /".to_string());
        }

        let type_ = type_parts[0].trim();
        let subtype = type_parts[1].trim();

        let mut parameters = HashMap::new();
        for part in &parts[1..] {
            if let Some(idx) = part.find('=') {
                let key = part[..idx].trim();
                let value = part[idx + 1..].trim();
                parameters.insert(key.to_string(), value.to_string());
            }
        }

        Ok(MimeType::with_parameters(type_, subtype, parameters))
    }
}

/// Comparator for MIME types that orders by specificity.
pub struct SpecificityComparator;

impl SpecificityComparator {
    /// Compare two MIME types by specificity.
    pub fn compare<T: AsRef<MimeType>>(mime_type1: T, mime_type2: T) -> Ordering {
        let mt1 = mime_type1.as_ref();
        let mt2 = mime_type2.as_ref();

        // Wildcard types are less specific
        if mt1.is_wildcard_type() && !mt2.is_wildcard_type() {
            return Ordering::Greater;
        }
        if mt2.is_wildcard_type() && !mt1.is_wildcard_type() {
            return Ordering::Less;
        }

        // Different types are incomparable
        if mt1.get_type() != mt2.get_type() {
            return Ordering::Equal;
        }

        // Wildcard subtypes are less specific
        if mt1.is_wildcard_subtype() && !mt2.is_wildcard_subtype() {
            return Ordering::Greater;
        }
        if mt2.is_wildcard_subtype() && !mt1.is_wildcard_subtype() {
            return Ordering::Less;
        }

        // Different subtypes are incomparable
        if mt1.get_subtype() != mt2.get_subtype() {
            return Ordering::Equal;
        }

        // More parameters = more specific
        mt2.get_parameters().len().cmp(&mt1.get_parameters().len())
    }
}

/// A `HashMap` with case-insensitive string keys, maintaining insertion order.
///
/// This map preserves the order in which elements were inserted, similar to
/// `LinkedHashMap`, but uses case-insensitive string comparison for keys.
#[derive(Clone)]
pub struct LinkedCaseInsensitiveMap<V> {
    // Store entries in insertion order using a linked list approach
    entries: LinkedList<(String, V)>,
    // Map from lowercase keys to indices for O(1) lookup
    indices: HashMap<String, usize>,
    locale: Option<String>, // Store locale info for case conversion
}

impl<V> LinkedCaseInsensitiveMap<V> {
    /// Creates a new empty `LinkedCaseInsensitiveMap`.
    pub fn new() -> Self {
        Self::with_locale(None)
    }

    /// Creates a new empty `LinkedCaseInsensitiveMap` with the specified locale.
    pub fn with_locale(locale: Option<String>) -> Self {
        LinkedCaseInsensitiveMap {
            entries: LinkedList::new(),
            indices: HashMap::new(),
            locale,
        }
    }

    /// Creates a new empty `LinkedCaseInsensitiveMap` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_locale(capacity, None)
    }

    /// Creates a new empty `LinkedCaseInsensitiveMap` with the specified capacity and locale.
    pub fn with_capacity_and_locale(capacity: usize, locale: Option<String>) -> Self {
        LinkedCaseInsensitiveMap {
            entries: LinkedList::new(),
            indices: HashMap::with_capacity(capacity),
            locale,
        }
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the locale used for case conversion.
    pub fn get_locale(&self) -> Option<&String> {
        self.locale.as_ref()
    }

    /// Converts a key to its case-insensitive form.
    fn convert_key(&self, key: &str) -> String {
        match &self.locale {
            Some(locale) if locale == "tr" || locale == "az" => {
                // Turkish/Azerbaijani special case handling
                // In a real implementation, this would use proper locale-aware case conversion
                key.to_lowercase()
            }
            _ => key.to_lowercase(),
        }
    }

    /// Removes a key from the indices map.
    #[allow(unused)]
    fn remove_case_insensitive_key(&mut self, key: &str) -> Option<String> {
        let converted = self.convert_key(key);
        self.indices.remove(&converted).map(|_| key.to_string())
    }

    /// Returns a reference to the value corresponding to the key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&V> {
        let converted = self.convert_key(key.as_ref());

        if let Some(&index) = self.indices.get(&converted) {
            // Find the entry at the given index
            for (i, (_k, v)) in self.entries.iter().enumerate() {
                if i == index {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Returns a mutable reference to the value corresponding to the key.
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut V> {
        let converted = self.convert_key(key.as_ref());

        if let Some(&index) = self.indices.get(&converted) {
            // Find the entry at the given index
            for (i, (_k, v)) in self.entries.iter_mut().enumerate() {
                if i == index {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: String, value: V) -> Option<V> {
        let converted = self.convert_key(&key);

        // Check if key already exists
        if let Some(&existing_index) = self.indices.get(&converted) {
            // Update existing entry
            for (i, (k, v)) in self.entries.iter_mut().enumerate() {
                if i == existing_index {
                    if k != &key {
                        // Key case differs, update the key
                        *k = key;
                    }
                    return Some(std::mem::replace(v, value));
                }
            }
        }

        // New entry
        self.entries.push_back((key, value));
        self.indices.insert(converted, self.entries.len() - 1);
        None
    }

    /// Removes a key from the map, returning the value at the key if the key was previously in the map.
    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<V>
    where
        V: Clone,
    {
        let converted = self.convert_key(key.as_ref());

        if let Some(index) = self.indices.remove(&converted) {
            // Remove the entry at the given index and adjust subsequent indices
            let mut remaining = LinkedList::new();
            let mut removed_value: Option<V> = None;

            for (i, (k, v)) in self.entries.iter().enumerate() {
                if i == index {
                    removed_value = Some(v.clone());
                    // Don't add this entry to remaining
                } else {
                    remaining.push_back((k.clone(), v.clone()));
                }
            }

            self.entries = remaining;

            // Rebuild indices
            self.indices.clear();
            for (i, (k, _)) in self.entries.iter().enumerate() {
                let converted = self.convert_key(k);
                self.indices.insert(converted, i);
            }

            removed_value
        } else {
            None
        }
    }

    /// Clears the map, removing all key-value pairs.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.indices.clear();
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        let converted = self.convert_key(key.as_ref());
        self.indices.contains_key(&converted)
    }

    /// Returns an iterator over the key-value pairs of the map in insertion order.
    pub fn iter(&self) -> Iter<'_, V> {
        Iter {
            inner: self.entries.iter(),
        }
    }

    /// Returns a mutable iterator over the key-value pairs of the map in insertion order.
    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut {
            inner: self.entries.iter_mut(),
        }
    }

    /// Returns an iterator over the keys of the map in insertion order.
    pub fn keys(&self) -> Keys<'_, V> {
        Keys {
            inner: self.entries.iter(),
        }
    }

    /// Returns an iterator over the values of the map in insertion order.
    pub fn values(&self) -> Values<'_, V> {
        Values {
            inner: self.entries.iter(),
        }
    }

    /// Returns a mutable iterator over the values of the map in insertion order.
    pub fn values_mut(&mut self) -> ValuesMut<'_, V> {
        ValuesMut {
            inner: self.entries.iter_mut(),
        }
    }
}

impl<V> Default for LinkedCaseInsensitiveMap<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> PartialEq for LinkedCaseInsensitiveMap<V>
where
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter()
            .all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
    }
}

impl<V> Eq for LinkedCaseInsensitiveMap<V> where V: Eq {}

impl<V> fmt::Debug for LinkedCaseInsensitiveMap<V>
where
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<V> Index<&str> for LinkedCaseInsensitiveMap<V> {
    type Output = V;

    fn index(&self, key: &str) -> &V {
        self.get(key).expect("no entry found for key")
    }
}

impl<V> IntoIterator for LinkedCaseInsensitiveMap<V> {
    type Item = (String, V);
    type IntoIter = IntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.entries.into_iter(),
        }
    }
}

impl<'a, V> IntoIterator for &'a LinkedCaseInsensitiveMap<V> {
    type Item = (&'a String, &'a V);
    type IntoIter = Iter<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, V> IntoIterator for &'a mut LinkedCaseInsensitiveMap<V> {
    type Item = (&'a String, &'a mut V);
    type IntoIter = IterMut<'a, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<V> FromIterator<(String, V)> for LinkedCaseInsensitiveMap<V> {
    fn from_iter<I: IntoIterator<Item = (String, V)>>(iter: I) -> Self {
        let mut map = LinkedCaseInsensitiveMap::new();
        for (key, value) in iter {
            map.insert(key, value);
        }
        map
    }
}

impl<V> Extend<(String, V)> for LinkedCaseInsensitiveMap<V> {
    fn extend<I: IntoIterator<Item = (String, V)>>(&mut self, iter: I) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

/// An iterator over the entries of a `LinkedCaseInsensitiveMap`.
pub struct Iter<'a, V> {
    inner: std::collections::linked_list::Iter<'a, (String, V)>,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a String, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// A mutable iterator over the entries of a `LinkedCaseInsensitiveMap`.
pub struct IterMut<'a, V> {
    inner: std::collections::linked_list::IterMut<'a, (String, V)>,
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (&'a String, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| (&*k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An owning iterator over the entries of a `LinkedCaseInsensitiveMap`.
pub struct IntoIter<V> {
    inner: std::collections::linked_list::IntoIter<(String, V)>,
}

impl<V> Iterator for IntoIter<V> {
    type Item = (String, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator over the keys of a `LinkedCaseInsensitiveMap`.
pub struct Keys<'a, V> {
    inner: std::collections::linked_list::Iter<'a, (String, V)>,
}

impl<'a, V> Iterator for Keys<'a, V> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator over the values of a `LinkedCaseInsensitiveMap`.
pub struct Values<'a, V> {
    inner: std::collections::linked_list::Iter<'a, (String, V)>,
}

impl<'a, V> Iterator for Values<'a, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// A mutable iterator over the values of a `LinkedCaseInsensitiveMap`.
pub struct ValuesMut<'a, V> {
    inner: std::collections::linked_list::IterMut<'a, (String, V)>,
}

impl<'a, V> Iterator for ValuesMut<'a, V> {
    type Item = &'a mut V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("Hello".to_string(), 1);

        assert_eq!(map.get("hello"), Some(&1));
        assert_eq!(map.get("HELLO"), Some(&1));
        assert_eq!(map.get("Hello"), Some(&1));
    }

    #[test]
    fn test_case_insensitive_keys() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("Key".to_string(), 1);
        map.insert("KEY".to_string(), 2); // Should update existing

        assert_eq!(map.len(), 1);
        assert_eq!(map.get("key"), Some(&2));
    }

    #[test]
    fn test_insertion_order() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("first".to_string(), 1);
        map.insert("second".to_string(), 2);
        map.insert("third".to_string(), 3);

        let mut iter = map.iter();
        assert_eq!(iter.next(), Some((&"first".to_string(), &1)));
        assert_eq!(iter.next(), Some((&"second".to_string(), &2)));
        assert_eq!(iter.next(), Some((&"third".to_string(), &3)));
    }

    #[test]
    fn test_remove() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("first".to_string(), 1);
        map.insert("second".to_string(), 2);

        assert_eq!(map.remove("FIRST"), Some(1));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("second"), Some(&2));
    }

    #[test]
    fn test_contains_key() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("test".to_string(), 42);

        assert!(map.contains_key("test"));
        assert!(map.contains_key("TEST"));
        assert!(!map.contains_key("missing"));
    }

    #[test]
    fn test_clear() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("key1".to_string(), 1);
        map.insert("key2".to_string(), 2);

        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_iteration() {
        let mut map = LinkedCaseInsensitiveMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);

        let keys: Vec<&String> = map.keys().collect();
        assert_eq!(keys, vec!["a", "b"]);

        let values: Vec<&i32> = map.values().collect();
        assert_eq!(values, vec![&1, &2]);
    }

    #[test]
    fn test_from_iterator() {
        let data = vec![
            ("key1".to_string(), 1),
            ("key2".to_string(), 2),
            ("KEY1".to_string(), 3), // Should update first entry
        ];

        let map: LinkedCaseInsensitiveMap<i32> = data.into_iter().collect();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("key1"), Some(&3));
        assert_eq!(map.get("key2"), Some(&2));
    }

    #[test]
    fn test_locale_specific() {
        // Turkish locale has special case for 'i' to uppercase 'İ'
        let mut map = LinkedCaseInsensitiveMap::with_locale(Some("tr".to_string()));
        map.insert("i".to_string(), 1);

        // In a full implementation with proper Unicode case folding,
        // this would work with Turkish special cases
        assert!(map.contains_key("i"));
    }
}
