use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use std::{borrow::Cow, collections::BTreeMap};

/// A JSON object backed by a `BTreeMap`, providing a fluent API for
/// reading, writing, and serializing key-value pairs.
#[derive(Clone, Debug)]
pub struct JsonObject {
    /// The underlying map storing JSON key-value pairs.
    entries: BTreeMap<Cow<'static, str>, Value>,
}

impl JsonObject {
    /// Creates a new, empty `JsonObject`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a `JsonObject` from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`JsonObjectError::ParseError`] if the input is not valid JSON
    /// or does not represent a JSON object.
    pub fn parse(json_str: &str) -> Result<Self, JsonObjectError> {
        Self::parse_object(json_str)
    }

    /// Parses a value of type `T` from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`JsonObjectError::ParseError`] if the input is not valid JSON
    /// or cannot be deserialized into `T`.
    pub fn parse_object<T: DeserializeOwned>(json_str: &str) -> Result<T, JsonObjectError> {
        serde_json::from_str::<T>(json_str).map_err(|e| JsonObjectError::ParseError(e.to_string()))
    }
}

impl JsonObject {
    /// Returns the number of key-value pairs.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if there are no key-value pairs.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns `true` if the object contains the given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Retrieves the value for `key` and deserializes it into `V`.
    ///
    /// Returns `None` if the key is missing or deserialization fails.
    pub fn get<V: DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.entries
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Retrieves the value for `key` and deserializes it into `V`,
    /// falling back to `default` if the key is missing or deserialization fails.
    pub fn get_or_default<V: DeserializeOwned>(&self, key: &str, default: V) -> V {
        self.entries
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(default)
    }

    /// Retrieves a floating-point value for `key`.
    pub fn get_double(&self, key: &str) -> Option<f64> {
        self.entries.get(key).and_then(|v| v.as_f64())
    }

    /// Retrieves a signed integer value for `key`.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.entries.get(key).and_then(|v| v.as_i64())
    }

    /// Retrieves an unsigned integer value for `key`.
    pub fn get_uint(&self, key: &str) -> Option<u64> {
        self.entries.get(key).and_then(|v| v.as_u64())
    }

    /// Retrieves a string value for `key`.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.entries.get(key).and_then(|v| v.as_str())
    }

    /// Retrieves a boolean value for `key`.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.entries.get(key).and_then(|v| v.as_bool())
    }

    /// Retrieves a JSON array for `key` and deserializes each element into `V`.
    ///
    /// Elements that fail to deserialize are silently skipped.
    /// Returns an empty vector if the key is missing or not an array.
    pub fn get_json_array<V: DeserializeOwned>(&self, key: &str) -> Vec<V> {
        self.entries
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Inserts a value for `key`, replacing any existing value, and returns
    /// `self` to support method chaining.
    pub fn set<V: Into<Value>>(mut self, key: impl Into<Cow<'static, str>>, value: V) -> Self {
        self.entries.insert(key.into(), value.into());
        self
    }

    /// Inserts a value for `key`, returning the previous value if any.
    pub fn set_opt<V: Into<Value>>(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: V,
    ) -> Option<Value> {
        self.entries.insert(key.into(), value.into())
    }

    /// Inserts a value for `key`, returning an error if the key is empty or
    /// already exists.
    pub fn put_once<V: Into<Value>>(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: V,
    ) -> Result<(), JsonObjectError> {
        let key = key.into();
        if key.is_empty() {
            return Err(JsonObjectError::KeyIsEmpty);
        }
        if self.entries.contains_key(&key) {
            return Err(JsonObjectError::KeyAlreadyExists);
        }
        self.set_opt(key, value);
        Ok(())
    }

    /// Inserts a value for `key`, returning an error if the key is empty or
    /// the value is JSON `null`.
    pub fn put_opt<V: Into<Value>>(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: V,
    ) -> Result<(), JsonObjectError> {
        let key = key.into();
        let value = value.into();
        if key.is_empty() {
            return Err(JsonObjectError::KeyIsEmpty);
        }
        if value.is_null() {
            return Err(JsonObjectError::KeyOrValueIsNull);
        }
        self.set_opt(key, value);
        Ok(())
    }

    /// Inserts all key-value pairs from an iterable collection.
    pub fn put_all<K, V>(&mut self, data: impl IntoIterator<Item = (K, V)>)
    where
        K: Into<Cow<'static, str>>,
        V: Into<Value>,
    {
        self.entries
            .extend(data.into_iter().map(|(k, v)| (k.into(), v.into())));
    }

    /// Returns a reference to the underlying `BTreeMap`.
    pub fn raw_value(&self) -> &BTreeMap<Cow<'static, str>, Value> {
        &self.entries
    }

    /// Removes all key-value pairs.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns an iterator over all values.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.values()
    }

    /// Returns an iterator over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &Cow<'static, str>> {
        self.entries.keys()
    }

    /// Serializes the object to a compact JSON string.
    ///
    /// Returns an empty string if serialization fails (which should not happen
    /// for a `BTreeMap<Cow<str>, Value>`).
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.entries).unwrap_or_default()
    }

    /// Serializes the object to a pretty-printed JSON string.
    pub fn to_json_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.entries).unwrap_or_default()
    }
}

impl Default for JsonObject {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

/// Errors that can occur during `JsonObject` operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JsonObjectError {
    /// The key is empty.
    KeyIsEmpty,
    /// The key already exists.
    KeyAlreadyExists,
    /// The value is JSON `null`.
    ValueIsNull,
    /// The key or value is JSON `null`.
    KeyOrValueIsNull,
    /// A parse error occurred.
    ParseError(String),
}

impl std::fmt::Display for JsonObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonObjectError::KeyIsEmpty => write!(f, "Key is empty"),
            JsonObjectError::KeyAlreadyExists => write!(f, "Key already exists"),
            JsonObjectError::ValueIsNull => write!(f, "Value is null"),
            JsonObjectError::KeyOrValueIsNull => write!(f, "Key or value is null"),
            JsonObjectError::ParseError(error) => {
                write!(f, "JsonObjectError::ParseError: {}", error)
            }
        }
    }
}

impl std::error::Error for JsonObjectError {}

impl From<serde_json::Error> for JsonObjectError {
    fn from(err: serde_json::Error) -> Self {
        JsonObjectError::ParseError(err.to_string())
    }
}

impl Serialize for JsonObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for (k, v) in &self.entries {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for JsonObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries: BTreeMap<Cow<'static, str>, Value> = Deserialize::deserialize(deserializer)?;
        Ok(JsonObject { entries })
    }
}

impl From<BTreeMap<Cow<'static, str>, Value>> for JsonObject {
    fn from(entries: BTreeMap<Cow<'static, str>, Value>) -> Self {
        Self { entries }
    }
}

impl From<JsonObject> for BTreeMap<Cow<'static, str>, Value> {
    fn from(obj: JsonObject) -> Self {
        obj.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let json_str = r#"{"name": "Alice", "age": 30}"#;
        let json_obj = JsonObject::parse(json_str).expect("parse error");

        assert_eq!(json_obj.get::<String>("name").unwrap(), "Alice");
        assert_eq!(json_obj.get::<i64>("age").unwrap(), 30);
    }

    #[test]
    fn test_new() {
        let json_obj = JsonObject::new();
        assert_eq!(json_obj.size(), 0);
        assert!(json_obj.is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let json_obj = JsonObject::new();
        let json_obj = json_obj.set("name", "Alice").set("age", 30);

        assert_eq!(json_obj.get::<String>("name").unwrap(), "Alice");
        assert_eq!(json_obj.get::<i64>("age").unwrap(), 30);
    }

    #[test]
    fn test_set_opt() {
        let mut json_obj = JsonObject::new();
        let old_value = json_obj.set_opt("name", "Alice");
        assert!(old_value.is_none());

        let old_value = json_obj.set_opt("name", "Bob");
        assert_eq!(old_value.unwrap(), serde_json::json!("Alice"));

        assert_eq!(json_obj.get::<String>("name").unwrap(), "Bob");
    }

    #[test]
    fn test_put_once() {
        let mut json_obj = JsonObject::new();
        assert!(json_obj.put_once("name", "Alice").is_ok());
        assert!(json_obj.put_once("name", "Bob").is_err());

        assert_eq!(json_obj.get::<String>("name").unwrap(), "Alice");
    }

    #[test]
    fn test_put_opt() {
        let mut json_obj = JsonObject::new();
        assert!(json_obj.put_opt("name", "Alice").is_ok());
        assert!(json_obj.put_opt("name", serde_json::Value::Null).is_err());

        assert_eq!(json_obj.get::<String>("name").unwrap(), "Alice");
    }

    #[test]
    fn test_parse_object() {
        let json_str = r#"{"name": "Alice", "age": 30}"#;
        let parsed: BTreeMap<String, Value> = JsonObject::parse_object(json_str).unwrap();

        assert_eq!(parsed["name"], serde_json::json!("Alice"));
        assert_eq!(parsed["age"], serde_json::json!(30));
    }

    #[test]
    fn test_to_json_string() {
        let json_obj = JsonObject::new();
        let json_str = json_obj
            .set("name", "Alice")
            .set("age", 30)
            .to_json_string();

        assert_eq!(json_str, r#"{"age":30,"name":"Alice"}"#);
    }

    #[test]
    fn test_to_json_string_pretty() {
        let json_obj = JsonObject::new();
        let json_str = json_obj
            .set("name", "Alice")
            .set("age", 30)
            .to_json_string_pretty();
        assert_eq!(json_str, "{\n  \"age\": 30,\n  \"name\": \"Alice\"\n}");
    }

    #[test]
    fn test_clear() {
        let json_obj = JsonObject::new();
        let mut json_obj = json_obj.set("name", "Alice").set("age", 30);
        json_obj.clear();

        assert_eq!(json_obj.size(), 0);
        assert!(json_obj.is_empty());
    }
}
