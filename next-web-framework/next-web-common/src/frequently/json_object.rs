use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{borrow::Cow, collections::BTreeMap};

use crate::error::json_object_error::JsonObjectError;

/// 定义一个可序列化和反序列化的 JSON 对象结构体。
///
/// The structure encapsulates a `BTreeMap` to store key-value pairs and provides various operation interfaces.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct JsonObject {
    /// 存储 JSON 键值对的核心数据结构。
    ///
    /// Core data structure storing JSON key-value pairs.
    raw_value: BTreeMap<Cow<'static, str>, Value>,
}

impl JsonObject {
    /// 创建一个新的 `JsonObject` 实例。
    ///
    /// Creates a new instance of `JsonObject`.
    ///
    /// # 返回值
    /// 返回一个初始化的 `JsonObject` 实例
    /// Returns an initialized `JsonObject` instance
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 JSON 字符串解析出指定类型的对象实例。
    ///
    /// Parses a specified type object from a JSON string.
    ///
    /// # 参数
    /// - `json_str`: 包含 JSON 数据的字符串。
    ///   - `json_str`: A string containing JSON data.
    ///
    /// # 返回值
    /// 如果解析成功，返回指定类型的对象；否则返回 `JsonObjectError::ParseError`。
    /// If parsing is successful, returns the specified type object; otherwise returns `JsonObjectError::ParseError`.
    pub fn parse_object<T: DeserializeOwned>(json_str: &str) -> Result<T, JsonObjectError> {
        serde_json::from_str::<T>(json_str).map_err(|_| JsonObjectError::ParseError)
    }
}

/// 为 `JsonObject` 提供功能接口。
impl JsonObject {
    /// 获取 `JsonObject` 中键值对的数量。
    ///
    /// Gets the number of key-value pairs in `JsonObject`.
    ///
    /// # 返回值
    /// 返回键值对的总数。
    /// Returns the total number of key-value pairs.
    pub fn size(&self) -> usize {
        self.raw_value.len()
    }

    /// 检查 `JsonObject` 是否为空。
    ///
    /// Checks if `JsonObject` is empty.
    ///
    /// # 返回值
    /// 如果没有任何键值对，返回 `true`；否则返回 `false`。
    /// Returns `true` if there are no key-value pairs; otherwise returns `false`.
    pub fn is_empty(&self) -> bool {
        self.raw_value.is_empty()
    }

    /// 检查 `JsonObject` 是否包含指定键。
    ///
    /// Checks if `JsonObject` contains the specified key.
    ///
    /// # 参数
    /// - `key`: 需要检查的键。
    ///   - `key`: The key to check.
    ///
    /// # 返回值
    /// 如果包含指定键，返回 `true`；否则返回 `false`。
    /// Returns `true` if the specified key exists; otherwise returns `false`.
    pub fn contains_key(&self, key: &str) -> bool {
        self.raw_value.contains_key(key)
    }

    /// 根据键获取值并反序列化为指定类型。
    ///
    /// Retrieves the value associated with the specified key and deserializes it into the specified type.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且反序列化成功，返回对应的值；否则返回 `None`。
    /// If the key exists and deserialization is successful, returns the corresponding value; otherwise returns `None`.
    pub fn get<V: DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.raw_value
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// 根据键获取值并反序列化为指定类型，如果键不存在则返回默认值。
    ///
    /// Retrieves the value associated with the specified key and deserializes it into the specified type.
    /// If the key does not exist, returns the default value.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    /// - `default`: 默认值。
    ///   - `default`: The default value.
    ///
    /// # 返回值
    /// 如果键存在且反序列化成功，返回对应的值；否则返回默认值。
    /// If the key exists and deserialization is successful, returns the corresponding value; otherwise returns the default value.
    pub fn get_or_default<V: DeserializeOwned>(&self, key: &str, default: V) -> V {
        if let Some(v) = self.raw_value.get(key) {
            return serde_json::from_value(v.clone()).unwrap_or(default);
        }
        default
    }

    /// 根据键获取浮点数值。
    ///
    /// Retrieves the floating-point value associated with the specified key.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为浮点数，返回对应的值；否则返回 `None`。
    /// If the key exists and the value is a floating-point number, returns the corresponding value; otherwise returns `None`.
    pub fn get_double(&self, key: &str) -> Option<f64> {
        self.raw_value.get(key).and_then(|v| v.as_f64())
    }

    /// 根据键获取整数值。
    ///
    /// Retrieves the integer value associated with the specified key.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为整数，返回对应的值；否则返回 `None`。
    /// If the key exists and the value is an integer, returns the corresponding value; otherwise returns `None`.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.raw_value.get(key).and_then(|v| v.as_i64())
    }

    /// 根据键获取无符号整数值。
    ///
    /// Retrieves the unsigned integer value associated with the specified key.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为无符号整数，返回对应的值；否则返回 `None`。
    /// If the key exists and the value is an unsigned integer, returns the corresponding value; otherwise returns `None`.
    pub fn get_uint(&self, key: &str) -> Option<u64> {
        self.raw_value.get(key).and_then(|v| v.as_u64())
    }

    /// 根据键获取字符串值。
    ///
    /// Retrieves the string value associated with the specified key.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为字符串，返回对应的值；否则返回 `None`。
    /// If the key exists and the value is a string, returns the corresponding value; otherwise returns `None`.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.raw_value.get(key).and_then(|v| v.as_str())
    }

    /// 根据键获取布尔值。
    ///
    /// Retrieves the boolean value associated with the specified key.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为布尔值，返回对应的值；否则返回 `None`。
    /// If the key exists and the value is a boolean, returns the corresponding value; otherwise returns `None`.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.raw_value.get(key).and_then(|v| v.as_bool())
    }

    /// 根据键获取 JSON 数组，并反序列化为指定类型。
    ///
    /// Retrieves the JSON array associated with the specified key and deserializes it into the specified type.
    ///
    /// # 参数
    /// - `key`: 需要查找的键。
    ///   - `key`: The key to look up.
    ///
    /// # 返回值
    /// 如果键存在且值为数组，返回反序列化后的列表；否则返回空列表。
    /// If the key exists and the value is an array, returns the deserialized list; otherwise returns an empty list.
    pub fn get_json_array<V: DeserializeOwned>(&self, key: &str) -> Vec<V> {
        self.raw_value
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| serde_json::from_value(v.clone()).unwrap())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 设置指定键的值，如果键已存在则覆盖原有值。
    ///
    /// Sets the value for the specified key, overwriting the existing value if the key already exists.
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    ///   - `key`: The key to set.
    /// - `value`: 需要设置的值。
    ///   - `value`: The value to set.
    ///
    /// # 返回值
    /// 返回自身以支持链式调用。
    /// Returns itself to support method chaining.
    pub fn set<V: Into<Value>>(mut self, key: impl Into<Cow<'static, str>>, value: V) -> Self {
        self.raw_value.insert(key.into(), value.into());
        self
    }

    /// 设置指定键的值，如果键已存在则覆盖原有值。
    ///
    /// Sets the value for the specified key, overwriting the existing value if the key already exists.
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    ///   - `key`: The key to set.
    /// - `value`: 需要设置的值。
    ///   - `value`: The value to set.
    ///
    /// # 返回值
    /// 如果键已存在，返回被覆盖的旧值；否则返回 `None`。
    /// If the key already exists, returns the old value that was replaced; otherwise returns `None`.
    pub fn set_opt<V: Into<Value>>(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: V,
    ) -> Option<Value> {
        self.raw_value.insert(key.into(), value.into())
    }

    /// 设置指定键的值，如果键已存在则返回错误。
    ///
    /// Sets the value for the specified key and returns an error if the key already exists.
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    ///   - `key`: The key to set.
    /// - `value`: 需要设置的值。
    ///   - `value`: The value to set.
    ///
    /// # 返回值
    /// 如果键不存在且设置成功，返回 `Ok(())`；否则返回相应的错误。
    /// If the key does not exist and setting is successful, returns `Ok(())`; otherwise returns the corresponding error.
    pub fn put_once<V: Into<Value>>(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: V,
    ) -> Result<(), JsonObjectError> {
        let key = key.into();

        if key.is_empty() {
            return Err(JsonObjectError::KeyIsEmpty);
        }
        if self.raw_value.contains_key(&key) {
            return Err(JsonObjectError::KeyAlreadyExists);
        }
        self.set_opt(key, value);
        Ok(())
    }

    /// 设置指定键的值，如果值为 `null` 则返回错误。
    ///
    /// Sets the value for the specified key and returns an error if the value is `null`.
    ///
    /// # 参数
    /// - `key`: 需要设置的键。
    ///   - `key`: The key to set.
    /// - `value`: 需要设置的值。
    ///   - `value`: The value to set.
    ///
    /// # 返回值
    /// 如果值不为 `null` 且设置成功，返回 `Ok(())`；否则返回相应的错误。
    /// If the value is not `null` and setting is successful, returns `Ok(())`; otherwise returns the corresponding error.
    pub fn put_opt<V: Into<Value>>(
        &mut self,
        key: impl Into<String>,
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

        self.set_opt(Cow::Owned(key), value);
        Ok(())
    }

    /// 将另一个可迭代的键值对集合中的所有键值对添加到 `JsonObject` 中。
    ///
    /// Adds all key-value pairs from another iterable collection to `JsonObject`.
    ///
    /// # 参数
    /// - `data`: 包含键值对的可迭代集合。
    ///   - `data`: An iterable collection containing key-value pairs.
    pub fn put_all(&mut self, data: impl IntoIterator<Item = (Cow<'static, str>, Value)>) {
        self.raw_value.extend(data);
    }

    /// 获取 `JsonObject` 的原始 `BTreeMap` 引用。
    ///
    /// Gets a reference to the underlying `BTreeMap`.
    ///
    /// # 返回值
    /// 返回底层 `BTreeMap` 的不可变引用。
    /// Returns an immutable reference to the underlying `BTreeMap`.
    pub fn raw_value(&self) -> &BTreeMap<Cow<'static, str>, Value> {
        &self.raw_value
    }

    /// 清空 `JsonObject` 中的所有键值对。
    ///
    /// Clears all key-value pairs in `JsonObject`.
    pub fn clear(&mut self) {
        self.raw_value.clear()
    }

    /// 获取 `JsonObject` 中所有值的引用列表。
    ///
    /// Gets a list of references to all values in `JsonObject`.
    ///
    /// # 返回值
    /// 返回所有值的引用列表。
    /// Returns a list of references to all values.
    pub fn values(&self) -> Vec<&Value> {
        self.raw_value.values().collect()
    }

    /// 获取 `JsonObject` 中所有键的引用列表。
    ///
    /// Gets a list of references to all keys in `JsonObject`.
    ///
    /// # 返回值
    /// 返回所有键的引用列表。
    /// Returns a list of references to all keys.
    pub fn keys(&self) -> Vec<&Cow<'static, str>> {
        self.raw_value.keys().collect()
    }

    /// 将 `JsonObject` 转换为 JSON 字符串。
    ///
    /// Converts `JsonObject` to a compact JSON string.
    ///
    /// # 返回值
    /// 返回紧凑格式的 JSON 字符串。
    /// Returns a compact JSON string.
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.raw_value).unwrap_or_default()
    }

    /// 将 `JsonObject` 转换为格式化的 JSON 字符串。
    ///
    /// Converts `JsonObject` to a pretty-printed JSON string.
    ///
    /// # 返回值
    /// 返回格式化的 JSON 字符串。
    /// Returns a pretty-printed JSON string.
    pub fn to_json_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.raw_value).unwrap_or_default()
    }
}

/// 实现 `Default` 特性以提供默认的 `JsonObject` 实例。
///
/// Implements the `Default` trait to provide a default `JsonObject` instance.
impl Default for JsonObject {
    /// 创建一个默认的 `JsonObject` 实例。
    ///
    /// Creates a default `JsonObject` instance.
    fn default() -> Self {
        Self {
            raw_value: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
